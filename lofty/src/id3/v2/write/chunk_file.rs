use crate::config::{ParseOptions, WriteOptions};
use crate::error::{LoftyError, Result};
use crate::iff::chunk::{Chunks, IFF_CHUNK_HEADER_SIZE};
use crate::macros::err;
use crate::util::io::{FileLike, Length, Truncate};

use std::io::{Cursor, Seek, SeekFrom, Write};

use byteorder::{ByteOrder, WriteBytesExt};

const CHUNK_NAME_UPPER: [u8; 4] = [b'I', b'D', b'3', b' '];
const CHUNK_NAME_LOWER: [u8; 4] = [b'i', b'd', b'3', b' '];

pub(in crate::id3::v2) fn write_to_chunk_file<F, B>(
	file: &mut F,
	tag: &[u8],
	write_options: WriteOptions,
) -> Result<()>
where
	F: FileLike,
	LoftyError: From<<F as Truncate>::Error>,
	LoftyError: From<<F as Length>::Error>,
	B: ByteOrder,
{
	const FIRST_CHUNK_LEN: u64 = (IFF_CHUNK_HEADER_SIZE as u64) + 4;

	// We only want to rely on the file size for the first chunk read.
	// Since a file can have trailing junk, but otherwise be valid, we actually want to use the
	// first chunk size, which (should) encompass the entire stream.
	let file_len = file.len()?;

	let mut chunks = Chunks::<_, B>::new(file, file_len);

	// TODO: Forcing the use of ParseOptions::default()
	let parse_options = ParseOptions::default();
	let Some(first_chunk) = chunks.next(parse_options.parsing_mode)? else {
		err!(UnknownFormat);
	};

	let mut actual_stream_size = first_chunk.size();

	let file = chunks.into_inner();
	file.rewind()?;

	let mut file_bytes = Cursor::new(Vec::with_capacity(actual_stream_size as usize));
	file.read_to_end(file_bytes.get_mut())?;

	if file_bytes.get_ref().len() < (actual_stream_size as usize + IFF_CHUNK_HEADER_SIZE as usize) {
		err!(SizeMismatch);
	}

	// The first chunk format is RIFF....WAVE
	file_bytes.seek(SeekFrom::Start(FIRST_CHUNK_LEN))?;

	let mut existing_id3_tag = None;

	let mut chunks = Chunks::<_, B>::new(file_bytes, u64::from(actual_stream_size));
	while let Some(chunk) = chunks.next(parse_options.parsing_mode)? {
		if chunk.fourcc == CHUNK_NAME_UPPER || chunk.fourcc == CHUNK_NAME_LOWER {
			// Need to add FIRST_CHUNK_LEN since we started the chunk reader at that offset
			let chunk_start = chunk.start() + FIRST_CHUNK_LEN;

			// Can't trust the written chunk size, since some encoders don't handle padding
			// correctly, see Chunks::skip(). Since skip detects invalid padding, we just let it
			// do the work and figure out where we are in the file afterwards.
			chunks.skip()?;

			let chunk_end = chunks.stream_position() + FIRST_CHUNK_LEN;

			log::debug!(
				"Found existing ID3v2 chunk, size: {} bytes",
				chunk_end - chunk_start
			);
			existing_id3_tag = Some(chunk_start..chunk_end);
			break;
		}
	}

	let mut file_bytes = chunks.into_inner();

	if let Some(existing_id3_tag) = existing_id3_tag {
		let tag_size = existing_id3_tag.end - existing_id3_tag.start;

		let _ = file_bytes
			.get_mut()
			.drain(existing_id3_tag.start as usize..existing_id3_tag.end as usize);

		actual_stream_size -= tag_size as u32;
	}

	if !tag.is_empty() {
		let mut tag_bytes = Cursor::new(Vec::new());
		if write_options.uppercase_id3v2_chunk {
			tag_bytes.write_all(&CHUNK_NAME_UPPER)?;
		} else {
			tag_bytes.write_all(&CHUNK_NAME_LOWER)?;
		}

		tag_bytes.write_u32::<B>(tag.len() as u32)?;
		tag_bytes.write_all(tag)?;

		// It is required an odd length chunk be padded with a 0
		// The 0 isn't included in the chunk size, however
		if !tag.len().is_multiple_of(2) {
			tag_bytes.write_u8(0)?;
		}

		let Ok(tag_size): std::result::Result<u32, _> = tag_bytes.get_ref().len().try_into() else {
			err!(TooMuchData)
		};

		let tag_position = actual_stream_size as usize + IFF_CHUNK_HEADER_SIZE as usize;

		file_bytes.get_mut().splice(
			tag_position..tag_position,
			tag_bytes.get_ref().iter().copied(),
		);

		actual_stream_size += tag_size;
	}

	file_bytes.seek(SeekFrom::Start(4))?;
	file_bytes.write_u32::<B>(actual_stream_size)?;

	file.rewind()?;
	file.truncate(0)?;
	file.write_all(file_bytes.get_ref())?;

	Ok(())
}
