use super::RIFFInfoListRef;
use crate::config::{ParsingMode, WriteOptions};
use crate::error::{
	FileEncodingError, FileParseError, SizeMismatchError, TagEncodingError, TooMuchDataError,
};
use crate::iff::chunk::{Chunks, IFF_CHUNK_HEADER_SIZE};
use crate::iff::error::ChunkParseError;
use crate::iff::wav::error::WavParseError;
use crate::iff::wav::read::verify_wav;
use crate::iff::wav::tag::error::RiffInfoListEncodingError;
use crate::io::VerifiedFile;
use crate::util::io::FileLike;

use std::borrow::Cow;
use std::io::{Cursor, Read, Seek, SeekFrom};

use byteorder::{LittleEndian, WriteBytesExt};

// TODO: Write JUNK chunk for padding
pub(in crate::iff::wav) fn write_riff_info<'a, F, I>(
	file: VerifiedFile<'_, F>,
	tag: &mut RIFFInfoListRef<'a, I>,
	write_options: WriteOptions,
) -> Result<(), FileEncodingError>
where
	F: FileLike,
	I: Iterator<Item = (&'a str, Cow<'a, str>)>,
{
	// The first chunk format is RIFF....WAVE
	const FIRST_CHUNK_LEN: u32 = IFF_CHUNK_HEADER_SIZE + 4;

	let file = file.into_inner();
	let original_stream_length = verify_wav(file).map_err(FileParseError::from)?;

	let mut riff_info_bytes = Vec::new();
	create_riff_info(&mut tag.items, &mut riff_info_bytes).map_err(TagEncodingError::from)?;

	file.rewind()?;

	let mut file_bytes = Cursor::new(Vec::new());
	file.read_to_end(file_bytes.get_mut())?;

	// File is lying about its length
	if file_bytes.get_ref().len() < (original_stream_length + IFF_CHUNK_HEADER_SIZE) as usize {
		return Err(SizeMismatchError.into());
	}

	file_bytes.seek(SeekFrom::Start(u64::from(FIRST_CHUNK_LEN)))?;

	let Some(original_info_list_size) = find_info_list(
		&mut file_bytes,
		u64::from(original_stream_length - 4),
		write_options.parse_options.parsing_mode,
	)
	.map_err(FileParseError::from)?
	else {
		// Simply append the info list to the end of the file and update the file size

		let new_stream_length = riff_info_bytes.len() as u64 + u64::from(original_stream_length);
		if new_stream_length > u64::from(u32::MAX) {
			return Err(TooMuchDataError.into());
		}

		file_bytes.rewind()?;

		let tag_position = (original_stream_length + IFF_CHUNK_HEADER_SIZE) as usize;
		file_bytes.seek(SeekFrom::Start(tag_position as u64))?;

		file_bytes
			.get_mut()
			.splice(tag_position..tag_position, riff_info_bytes.iter().copied());

		file_bytes.seek(SeekFrom::Start(4))?;
		file_bytes.write_u32::<LittleEndian>(new_stream_length as u32)?;

		file.rewind()?;
		file.truncate(0)?;
		file.write_all(file_bytes.get_ref())?;

		return Ok(());
	};

	// Replace the existing tag

	let info_list_start = file_bytes.seek(SeekFrom::Current(-12))? as usize;

	// `original_info_list_size` doesn't include the b"LIST\0\0\0\0" chunk header
	let info_list_end =
		info_list_start + (IFF_CHUNK_HEADER_SIZE + original_info_list_size) as usize;
	let original_info_list = info_list_start..info_list_end;

	let new_stream_length = riff_info_bytes.len() as u64
		+ (u64::from(original_stream_length) - original_info_list.len() as u64);
	if new_stream_length > u64::from(u32::MAX) {
		return Err(TooMuchDataError.into());
	}

	let _ = file_bytes
		.get_mut()
		.splice(original_info_list, riff_info_bytes);

	let _ = file_bytes
		.get_mut()
		.splice(4..8, (new_stream_length as u32).to_le_bytes());

	file.rewind()?;
	file.truncate(0)?;
	file.write_all(file_bytes.get_ref())?;

	Ok(())
}

fn find_info_list<R>(
	data: &mut R,
	file_size: u64,
	parse_mode: ParsingMode,
) -> Result<Option<u32>, WavParseError>
where
	R: Read + Seek,
{
	let mut info = None;

	let mut chunks = Chunks::<_, LittleEndian>::new(data, file_size);
	while let Some(mut chunk) = chunks.next(parse_mode)? {
		if &chunk.fourcc != b"LIST" {
			continue;
		}

		let mut list_type = [0; 4];
		chunk
			.read_exact(&mut list_type)
			.map_err(|e| ChunkParseError::from(e).with_fourcc(chunk.fourcc))?;

		if &list_type == b"INFO" {
			log::debug!(
				"Found existing RIFF INFO list, size: {} bytes",
				chunk.size()
			);

			info = Some(chunk.size());
			break;
		}
	}

	Ok(info)
}

pub(super) fn create_riff_info(
	items: &mut dyn Iterator<Item = (&str, Cow<'_, str>)>,
	bytes: &mut Vec<u8>,
) -> Result<(), RiffInfoListEncodingError> {
	let mut items = items.peekable();

	if items.peek().is_none() {
		log::debug!("No items to write, removing RIFF INFO list");
		return Ok(());
	}

	bytes.extend(b"LIST\0\0\0\0INFO");
	for (k, v) in items {
		if v.is_empty() {
			continue;
		}

		let val_b = v.as_bytes();
		// Account for null terminator
		let len = val_b.len() + 1;

		// Each value has to be null terminated and have an even length
		let terminator: &[u8] = if len % 2 == 0 { &[0] } else { &[0, 0] };

		bytes.extend(k.as_bytes());
		bytes.extend(&(len as u32).to_le_bytes());
		bytes.extend(val_b);
		bytes.extend(terminator);
	}

	let list_size = Vec::len(bytes) - IFF_CHUNK_HEADER_SIZE as usize;
	if list_size > u32::MAX as usize {
		return Err(TooMuchDataError.into());
	}

	log::debug!("Created RIFF INFO list, size: {} bytes", list_size);
	bytes[4..8].copy_from_slice(&(list_size as u32).to_le_bytes());

	Ok(())
}
