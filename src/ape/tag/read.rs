use super::item::ApeItem;
use super::ApeTag;
use crate::ape::constants::INVALID_KEYS;
use crate::ape::header::ApeHeader;
use crate::error::{ErrorKind, FileDecodingError, LoftyError, Result};
use crate::file::FileType;
use crate::macros::try_vec;
use crate::tag::item::ItemValue;

use std::io::{Read, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};

pub(crate) fn read_ape_tag<R>(data: &mut R, header: ApeHeader) -> Result<ApeTag>
where
	R: Read + Seek,
{
	let mut tag = ApeTag::default();
	let mut remaining_size = header.size;

	for _ in 0..header.item_count {
		if remaining_size < 11 {
			break;
		}

		let value_size = data.read_u32::<LittleEndian>()?;
		if value_size > remaining_size {
			return Err(LoftyError::new(ErrorKind::TooMuchData));
		}

		remaining_size -= 4;
		let flags = data.read_u32::<LittleEndian>()?;

		let mut key = Vec::new();
		let mut key_char = data.read_u8()?;

		while key_char != 0 {
			key.push(key_char);
			key_char = data.read_u8()?;
		}

		let key = String::from_utf8(key).map_err(|_| {
			FileDecodingError::new(FileType::APE, "APE tag item contains a non UTF-8 key")
		})?;

		if INVALID_KEYS.contains(&&*key.to_uppercase()) {
			return Err(FileDecodingError::new(
				FileType::APE,
				"APE tag item contains an illegal key",
			)
			.into());
		}

		let read_only = (flags & 1) == 1;
		let item_type = (flags >> 1) & 3;

		// TODO: This could use a warning
		if value_size == 0 || key.len() < 2 || key.len() > 255 {
			continue;
		}

		let mut value = try_vec![0; value_size as usize];
		data.read_exact(&mut value)?;

		let parsed_value = match item_type {
			0 => ItemValue::Text(String::from_utf8(value).map_err(|_| {
				FileDecodingError::new(
					FileType::APE,
					"Failed to convert text item into a UTF-8 string",
				)
			})?),
			1 => ItemValue::Binary(value),
			2 => ItemValue::Locator(String::from_utf8(value).map_err(|_| {
				FileDecodingError::new(
					FileType::APE,
					"Failed to convert locator item into a UTF-8 string",
				)
			})?),
			_ => {
				return Err(FileDecodingError::new(
					FileType::APE,
					"APE tag item contains an invalid item type",
				)
				.into())
			},
		};

		let mut item = ApeItem::new(key, parsed_value)?;

		if read_only {
			item.read_only = true;
		}

		tag.insert(item);
	}

	// Skip over footer
	data.seek(SeekFrom::Current(32))?;

	Ok(tag)
}
