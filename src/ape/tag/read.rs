use super::ape_tag::ApeTag;
use super::item::ApeItem;
use crate::ape::header::ApeHeader;
use crate::ape::constants::INVALID_KEYS;
use crate::error::{FileDecodingError, Result};
use crate::types::file::FileType;
use crate::types::item::ItemValue;
use crate::macros::try_vec;

use std::io::{Read, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};

pub(crate) fn read_ape_tag<R>(data: &mut R, header: ApeHeader) -> Result<ApeTag>
where
	R: Read + Seek,
{
	let mut tag = ApeTag::default();

	for _ in 0..header.item_count {
		let value_size = data.read_u32::<LittleEndian>()?;

		if value_size == 0 {
			return Err(FileDecodingError::new(
				FileType::APE,
				"APE tag item value has an invalid size (0)",
			)
			.into());
		}

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

		let item_type = (flags & 6) >> 1;

		let mut value = try_vec![0; value_size as usize];
		data.read_exact(&mut value)?;

		let parsed_value = match item_type {
			0 => ItemValue::Text(String::from_utf8(value).map_err(|_| {
				FileDecodingError::new(
					FileType::APE,
					"Expected a string value based on flags, found binary data",
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
