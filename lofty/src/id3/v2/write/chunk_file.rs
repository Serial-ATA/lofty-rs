use crate::config::{ParseOptions, WriteOptions};
use crate::error::{
	FileEncodingError, FileParseError, SizeMismatchError, TagEncodingError, TooMuchDataError,
	UnknownFormatError,
};
use crate::file::FileType;
use crate::iff::chunk::{Chunks, IFF_CHUNK_HEADER_SIZE};
use crate::iff::error::ChunkParseError;
use crate::io::VerifiedFile;
use crate::tag::TagType;
use crate::util::io::FileLike;

use std::io::{Cursor, Seek, SeekFrom, Write};
use std::ops::Range;

use byteorder::{ByteOrder, WriteBytesExt};

const CHUNK_NAME_UPPER: [u8; 4] = *b"ID3 ";
const CHUNK_NAME_LOWER: [u8; 4] = *b"id3 ";
const JUNK_CHUNK_NAME: [u8; 4] = *b"JUNK";

fn handle_chunk_error(format: FileType, error: ChunkParseError) -> FileParseError {
	FileParseError::new(format, error.into())
}

fn write_id3v2_chunk<W, B>(
	tag: &[u8],
	write_options: WriteOptions,
	writer: &mut W,
) -> Result<(), FileEncodingError>
where
	W: Write,
	B: ByteOrder,
{
	if write_options.uppercase_id3v2_chunk {
		writer.write_all(&CHUNK_NAME_UPPER)?;
	} else {
		writer.write_all(&CHUNK_NAME_LOWER)?;
	}

	writer.write_u32::<B>(tag.len() as u32)?;
	writer.write_all(tag)?;

	// It is required an odd length chunk be padded with a 0
	// The 0 isn't included in the chunk size, however
	if !tag.len().is_multiple_of(2) {
		writer.write_u8(0)?;
	}

	Ok(())
}

pub(in crate::id3::v2) fn write_to_chunk_file<F, B>(
	file: VerifiedFile<'_, F>,
	tag: &[u8],
	write_options: WriteOptions,
) -> Result<(), FileEncodingError>
where
	F: FileLike,
	B: ByteOrder,
{
	let mut tag_chunk_size;
	if tag.is_empty() {
		tag_chunk_size = 0u64;
	} else {
		tag_chunk_size = (tag.len() + IFF_CHUNK_HEADER_SIZE as usize)
			.try_into()
			.map_err(|_| TagEncodingError::new(TagType::Id3v2, TooMuchDataError.into()))?;
	}

	if !tag.len().is_multiple_of(2) {
		tag_chunk_size += 1;
	}

	let format = file.format();
	let file = file.into_inner();

	// We only want to rely on the file size for the first chunk read.
	// Since a file can have trailing junk, but otherwise be valid, we actually want to use the
	// first chunk size, which (should) encompass the entire stream.
	let file_len = file.len()?;

	// TODO: Forcing ParseOptions::default()
	let parse_options = ParseOptions::default();

	let mut file_context = find_existing_id3v2_tag::<_, B>(file, file_len, format, parse_options)?;
	if let Some(existing_id3_tag) = file_context.existing_id3_tag.clone() {
		let existing_tag_len = existing_id3_tag.end - existing_id3_tag.start;

		// We can just overwrite tags at the end of the file
		if existing_id3_tag.end == file_context.stream_length
			&& file_len == file_context.stream_length
		{
			let mut updated_stream_len = file_context.stream_length;
			if tag.is_empty() {
				updated_stream_len -= existing_tag_len;
				file.truncate(existing_id3_tag.start)?;
			} else {
				file.seek(SeekFrom::Start(existing_id3_tag.start))?;
				write_id3v2_chunk::<_, B>(tag, write_options, file)?;

				if existing_tag_len > tag_chunk_size {
					let remainder = existing_tag_len - tag_chunk_size;
					updated_stream_len -= remainder;
					file.truncate(existing_id3_tag.end - remainder)?;
				} else {
					updated_stream_len -= tag_chunk_size - existing_tag_len;
				}
			}

			// Update RIFF chunk size
			file.seek(SeekFrom::Start(4))?;
			file.write_u32::<B>((updated_stream_len as u32) - IFF_CHUNK_HEADER_SIZE)?;

			return Ok(());
		}

		// WAV has a `JUNK` chunk we can use to avoid rewriting the whole file.
		// Unfortunately, AIFF doesn't have an equivalent, so we'll always have to rewrite.
		if format == FileType::Wav && write_options.preferred_padding.is_some() {
			if tag.is_empty() {
				log::debug!("No items to write, removing ID3v2 tag");

				file.seek(SeekFrom::Start(existing_id3_tag.start))?;
				file.write_all(&JUNK_CHUNK_NAME)?;

				// From the spec: "A JUNK chunk represents padding, filler or outdated information.
				// It contains no relevant data; it is a space filler of arbitrary size."
				//
				// So there's no need to overwrite the actual tag, we can just leave it in place with
				// the `JUNK` FourCC.
				return Ok(());
			}

			// - `IFF_CHUNK_HEADER_SIZE`, since we need at least `IFF_CHUNK_HEADER_SIZE` bytes to
			// write a `JUNK` chunk
			if (existing_tag_len - u64::from(IFF_CHUNK_HEADER_SIZE)) >= tag_chunk_size {
				let remainder = existing_tag_len - tag_chunk_size;
				log::trace!("Existing tag large enough to overwrite (padding size: {remainder})");

				file.seek(SeekFrom::Start(existing_id3_tag.start))?;
				write_id3v2_chunk::<_, B>(tag, write_options, file)?;

				file.write_all(&JUNK_CHUNK_NAME)?;
				file.write_u32::<B>(remainder as u32)?;
				return Ok(());
			}
		}

		log::debug!("Existing ID3v2 tag too small to overwrite, rewriting whole file");
	} else if tag.is_empty() {
		log::debug!("No tag to overwrite, nothing to do");
		return Ok(());
	}

	let mut file_bytes = Cursor::new(Vec::with_capacity(file_context.stream_length as usize));
	file.rewind()?;
	file.read_to_end(file_bytes.get_mut())?;

	let mut tag_bytes = Vec::new();
	if !tag.is_empty() {
		write_id3v2_chunk::<_, B>(tag, write_options, &mut tag_bytes)?;
	}

	file_bytes.get_mut().splice(
		file_context.stream_length as usize..file_context.stream_length as usize,
		tag_bytes.iter().copied(),
	);

	if let Some(existing_id3_tag) = file_context.existing_id3_tag {
		file_context.stream_length -= existing_id3_tag.end - existing_id3_tag.start;
		file_bytes
			.get_mut()
			.drain(existing_id3_tag.start as usize..existing_id3_tag.end as usize);
	}

	file_context.stream_length += tag_chunk_size;

	// Update RIFF chunk size
	file_bytes.seek(SeekFrom::Start(4))?;
	file_bytes.write_u32::<B>((file_context.stream_length as u32) - IFF_CHUNK_HEADER_SIZE)?;

	file.rewind()?;
	file.truncate(0)?;
	file.write_all(file_bytes.get_ref())?;

	Ok(())
}

struct IffFileContext {
	stream_length: u64,
	existing_id3_tag: Option<Range<u64>>,
}

fn find_existing_id3v2_tag<F, B: ByteOrder>(
	file: &mut F,
	file_len: u64,
	format: FileType,
	parse_options: ParseOptions,
) -> Result<IffFileContext, FileParseError>
where
	F: FileLike,
{
	// RIFF....WAVE
	const FIRST_CHUNK_LEN: u64 = (IFF_CHUNK_HEADER_SIZE as u64) + 4;

	let mut chunks = Chunks::<_, B>::new(file, file_len);

	let Some(first_chunk) = chunks
		.next(parse_options.parsing_mode)
		.map_err(|e| handle_chunk_error(format, e))?
	else {
		return Err(UnknownFormatError.into());
	};

	let actual_stream_size = u64::from(first_chunk.size()) + u64::from(IFF_CHUNK_HEADER_SIZE);

	if file_len < actual_stream_size {
		return Err(SizeMismatchError.into());
	}

	if file_len > actual_stream_size {
		log::warn!(
			"File size is larger than expected stream size ({file_len} bytes vs \
			 {actual_stream_size} bytes), there may be trailing junk data!"
		);
	}

	let mut context = IffFileContext {
		stream_length: actual_stream_size,
		existing_id3_tag: None,
	};

	let file = chunks.into_inner();
	file.seek(SeekFrom::Start(FIRST_CHUNK_LEN))?;

	let mut chunks = Chunks::<_, B>::new(
		file,
		context.stream_length - u64::from(IFF_CHUNK_HEADER_SIZE),
	);
	while let Some(chunk) = chunks
		.next(parse_options.parsing_mode)
		.map_err(|e| handle_chunk_error(format, e))?
	{
		if chunk.fourcc == CHUNK_NAME_UPPER || chunk.fourcc == CHUNK_NAME_LOWER {
			// Need to add FIRST_CHUNK_LEN since we started the chunk reader at that offset
			let chunk_start = chunk.start() + FIRST_CHUNK_LEN;

			// Can't trust the written chunk size, since some encoders don't handle padding
			// correctly, see Chunks::skip(). Since skip detects invalid padding, we just let it
			// do the work and figure out where we are in the file afterwards.
			chunks.skip().map_err(|e| handle_chunk_error(format, e))?;

			let chunk_end = chunks.stream_position() + FIRST_CHUNK_LEN;

			log::debug!(
				"Found existing ID3v2 chunk, size: {} bytes",
				chunk_end - chunk_start
			);
			context.existing_id3_tag = Some(chunk_start..chunk_end);
			break;
		}
	}

	Ok(context)
}
