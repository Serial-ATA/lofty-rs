use super::read::verify_wav;
use crate::error::{LoftyError, Result};
use crate::types::tag::{ItemValue, Tag, TagType};

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

fn find_info_list<T>(data: &mut T) -> Result<bool>
where
	T: Read + Seek,
{
	let mut fourcc = [0; 4];

	let mut found_info = false;

	while let (Ok(()), Ok(size)) = (
		data.read_exact(&mut fourcc),
		data.read_u32::<LittleEndian>(),
	) {
		if &fourcc == b"LIST" {
			let mut list_type = [0; 4];
			data.read_exact(&mut list_type)?;

			if &list_type == b"INFO" {
				data.seek(SeekFrom::Current(-8))?;
				found_info = true;
				break;
			}

			data.seek(SeekFrom::Current(-8))?;
		}

		data.seek(SeekFrom::Current(i64::from(size)))?;
	}

	Ok(found_info)
}

// TODO: ID3v2
pub(crate) fn write_to(data: &mut File, tag: &Tag) -> Result<()> {
	if tag.tag_type() != &TagType::RiffInfo {
		return Err(LoftyError::UnsupportedTag);
	}

	verify_wav(data)?;

	let mut riff_info_bytes = Vec::new();
	create_riff_info(tag, &mut riff_info_bytes)?;

	if find_info_list(data)? {
		let info_list_size = data.read_u32::<LittleEndian>()? as usize;
		data.seek(SeekFrom::Current(-8))?;

		let info_list_start = data.seek(SeekFrom::Current(0))? as usize;
		let info_list_end = info_list_start + 8 + info_list_size;

		data.seek(SeekFrom::Start(0))?;

		let mut file_bytes = Vec::new();
		data.read_to_end(&mut file_bytes)?;

		let _ = file_bytes.splice(info_list_start..info_list_end, riff_info_bytes);

		let total_size = (file_bytes.len() - 8) as u32;
		let _ = file_bytes.splice(4..8, total_size.to_le_bytes());

		data.seek(SeekFrom::Start(0))?;
		data.set_len(0)?;
		data.write_all(&*file_bytes)?;
	} else {
		data.seek(SeekFrom::End(0))?;

		data.write_all(&riff_info_bytes)?;

		let len = (data.seek(SeekFrom::Current(0))? - 8) as u32;

		data.seek(SeekFrom::Start(4))?;
		data.write_u32::<LittleEndian>(len)?;
	}

	Ok(())
}

fn create_riff_info(tag: &Tag, bytes: &mut Vec<u8>) -> Result<()> {
	if tag.item_count() == 0 {
		return Ok(());
	}

	bytes.extend(b"LIST".iter());
	bytes.extend(b"INFO".iter());

	for item in tag.items() {
		if let Some(key) = item.key().map_key(&TagType::RiffInfo) {
			if key.len() == 4 && key.is_ascii() {
				if let ItemValue::Text(value) = item.value() {
					if value.is_empty() {
						continue;
					}

					let val_b = value.as_bytes();
					// Account for null terminator
					let len = val_b.len() + 1;

					// Each value has to be null terminated and have an even length
					let (size, terminator): (u32, &[u8]) = if len % 2 == 0 {
						(len as u32, &[0])
					} else {
						((len + 1) as u32, &[0, 0])
					};

					bytes.extend(key.as_bytes().iter());
					bytes.extend(size.to_le_bytes().iter());
					bytes.extend(val_b.iter());
					bytes.extend(terminator.iter());
				}
			}
		}
	}

	let packet_size = bytes.len() - 4;

	if packet_size > u32::MAX as usize {
		return Err(LoftyError::TooMuchData);
	}

	let size = (packet_size as u32).to_le_bytes();

	#[allow(clippy::needless_range_loop)]
	for i in 0..4 {
		bytes.insert(i + 4, size[i]);
	}

	Ok(())
}
