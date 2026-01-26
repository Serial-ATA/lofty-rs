use super::RIFFInfoListRef;
use crate::config::WriteOptions;
use crate::error::{LoftyError, Result};
use crate::iff::chunk::Chunks;
use crate::iff::wav::read::verify_wav;
use crate::macros::err;
use crate::util::io::{FileLike, Length, Truncate};

use std::borrow::Cow;
use std::io::{Cursor, Read, Seek, SeekFrom};

use byteorder::{LittleEndian, WriteBytesExt};

const RIFF_CHUNK_HEADER_SIZE: usize = 8;

pub(in crate::iff::wav) fn write_riff_info<'a, F, I>(
	file: &mut F,
	tag: &mut RIFFInfoListRef<'a, I>,
	_write_options: WriteOptions,
) -> Result<()>
where
	F: FileLike,
	LoftyError: From<<F as Truncate>::Error>,
	LoftyError: From<<F as Length>::Error>,
	I: Iterator<Item = (&'a str, Cow<'a, str>)>,
{
	let mut stream_length = verify_wav(file)?;

	let mut riff_info_bytes = Vec::new();
	create_riff_info(&mut tag.items, &mut riff_info_bytes)?;

	file.rewind()?;

	let mut file_bytes = Cursor::new(Vec::new());
	file.read_to_end(file_bytes.get_mut())?;

	if file_bytes.get_ref().len() < (stream_length as usize + RIFF_CHUNK_HEADER_SIZE) {
		err!(SizeMismatch);
	}

	// The first chunk format is RIFF....WAVE
	file_bytes.seek(SeekFrom::Start(12))?;

	let Some(info_list_size) = find_info_list(&mut file_bytes, u64::from(stream_length - 4))?
	else {
		// Simply append the info list to the end of the file and update the file size
		file_bytes.rewind()?;

		let tag_position = stream_length as usize + RIFF_CHUNK_HEADER_SIZE;

		file_bytes.seek(SeekFrom::Start(tag_position as u64))?;

		file_bytes
			.get_mut()
			.splice(tag_position..tag_position, riff_info_bytes.iter().copied());

		let len = (riff_info_bytes.len() + tag_position - 8) as u32;

		file_bytes.seek(SeekFrom::Start(4))?;
		file_bytes.write_u32::<LittleEndian>(len)?;

		file.rewind()?;
		file.truncate(0)?;
		file.write_all(file_bytes.get_ref())?;

		return Ok(());
	};

	// Replace the existing tag

	let info_list_start = file_bytes.seek(SeekFrom::Current(-12))? as usize;
	let info_list_end = info_list_start + RIFF_CHUNK_HEADER_SIZE + info_list_size as usize;

	stream_length -= info_list_end as u32 - info_list_start as u32;

	let new_tag_len = riff_info_bytes.len() as u32;
	let _ = file_bytes
		.get_mut()
		.splice(info_list_start..info_list_end, riff_info_bytes);

	stream_length += new_tag_len;

	let _ = file_bytes
		.get_mut()
		.splice(4..8, stream_length.to_le_bytes());

	file.rewind()?;
	file.truncate(0)?;
	file.write_all(file_bytes.get_ref())?;

	Ok(())
}

fn find_info_list<R>(data: &mut R, file_size: u64) -> Result<Option<u32>>
where
	R: Read + Seek,
{
	let mut info = None;

	let mut chunks = Chunks::<LittleEndian>::new(file_size);

	while let Ok(true) = chunks.next(data) {
		if &chunks.fourcc == b"LIST" {
			let mut list_type = [0; 4];
			data.read_exact(&mut list_type)?;

			if &list_type == b"INFO" {
				log::debug!("Found existing RIFF INFO list, size: {} bytes", chunks.size);

				info = Some(chunks.size);
				break;
			}

			data.seek(SeekFrom::Current(-8))?;
		}

		chunks.skip(data)?;
	}

	Ok(info)
}

pub(super) fn create_riff_info(
	items: &mut dyn Iterator<Item = (&str, Cow<'_, str>)>,
	bytes: &mut Vec<u8>,
) -> Result<()> {
	let mut items = items.peekable();

	if items.peek().is_none() {
		log::debug!("No items to write, removing RIFF INFO list");
		return Ok(());
	}

	bytes.extend(b"LIST");
	bytes.extend(b"INFO");

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

	let packet_size = Vec::len(bytes) - 4;

	if packet_size > u32::MAX as usize {
		err!(TooMuchData);
	}

	log::debug!("Created RIFF INFO list, size: {} bytes", packet_size);
	let size = (packet_size as u32).to_le_bytes();

	#[allow(clippy::needless_range_loop)]
	for i in 0..4 {
		bytes.insert(i + 4, size[i]);
	}

	Ok(())
}
