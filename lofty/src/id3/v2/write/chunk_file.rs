use crate::config::WriteOptions;
use crate::error::{LoftyError, Result};
use crate::iff::chunk::Chunks;
use crate::macros::err;
use crate::util::io::{FileLike, Length, Truncate};

use std::io::{Cursor, Seek, SeekFrom, Write};

use byteorder::{ByteOrder, WriteBytesExt};

const CHUNK_NAME_UPPER: [u8; 4] = [b'I', b'D', b'3', b' '];
const CHUNK_NAME_LOWER: [u8; 4] = [b'i', b'd', b'3', b' '];
const RIFF_CHUNK_HEADER_SIZE: usize = 8;

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
	// Only rely on the actual file for the first chunk read
	let file_len = file.len()?;

	let mut chunks = Chunks::<B>::new(file_len);
	chunks.next(file)?;

	let mut actual_stream_size = chunks.size;

	file.rewind()?;

	let mut file_bytes = Cursor::new(Vec::with_capacity(actual_stream_size as usize));
	file.read_to_end(file_bytes.get_mut())?;

	if file_bytes.get_ref().len() < (actual_stream_size as usize + RIFF_CHUNK_HEADER_SIZE) {
		err!(SizeMismatch);
	}

	// The first chunk format is RIFF....WAVE
	file_bytes.seek(SeekFrom::Start(12))?;

	let (mut exising_id3_start, mut existing_id3_size) = (None, None);

	let mut chunks = Chunks::<B>::new(u64::from(actual_stream_size));
	while let Ok(true) = chunks.next(&mut file_bytes) {
		if chunks.fourcc == CHUNK_NAME_UPPER || chunks.fourcc == CHUNK_NAME_LOWER {
			exising_id3_start = Some(file_bytes.stream_position()? - 8);
			existing_id3_size = Some(chunks.size);
			break;
		}

		chunks.skip(&mut file_bytes)?;
	}

	if let (Some(exising_id3_start), Some(mut existing_id3_size)) =
		(exising_id3_start, existing_id3_size)
	{
		// We need to remove the padding byte if it exists
		if existing_id3_size % 2 != 0 {
			existing_id3_size += 1;
		}

		let existing_tag_end =
			exising_id3_start as usize + RIFF_CHUNK_HEADER_SIZE + existing_id3_size as usize;
		let _ = file_bytes
			.get_mut()
			.drain(exising_id3_start as usize..existing_tag_end);

		actual_stream_size -= existing_id3_size + RIFF_CHUNK_HEADER_SIZE as u32;
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
		if tag.len() % 2 != 0 {
			tag_bytes.write_u8(0)?;
		}

		let Ok(tag_size): std::result::Result<u32, _> = tag_bytes.get_ref().len().try_into() else {
			err!(TooMuchData)
		};

		let tag_position = actual_stream_size as usize + RIFF_CHUNK_HEADER_SIZE;

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
