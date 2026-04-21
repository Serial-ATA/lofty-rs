use super::ApeTag;
use super::item::ApeItem;
use crate::ape::APE_PICTURE_TYPES;
use crate::ape::constants::APE_PREAMBLE;
use crate::ape::error::ApeTagParseError;
use crate::ape::header::{self, ApeHeader};
use crate::ape::tag::error::ApeTagItemParseError;
use crate::config::ParseOptions;
use crate::error::SizeMismatchError;
use crate::macros::try_vec;
use crate::tag::ItemValue;
use crate::util::text::utf8_decode;

use std::io::{Read, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};

/// Attempt to read an APE tag from the current position with the provided header
///
/// This assumes that the reader immediately follows the end of the header
pub(crate) fn read_ape_tag_with_header<R>(
	reader: &mut R,
	header: ApeHeader,
	parse_options: ParseOptions,
) -> Result<ApeTag, ApeTagParseError>
where
	R: Read + Seek,
{
	fn parse_item_with_key<R>(
		reader: &mut R,
		key: String,
		flags: u32,
		value_size: u32,
	) -> Result<ApeItem, ApeTagItemParseError>
	where
		R: Read + Seek,
	{
		let read_only = (flags & 1) == 1;
		let item_type = (flags >> 1) & 3;

		let mut value = match try_vec![0; value_size as usize] {
			Ok(v) => v,
			Err(e) => return Err((key, e).into()),
		};

		if let Err(e) = reader.read_exact(&mut value) {
			return Err((key, e).into());
		}

		let parsed_value = match item_type {
			0 => ItemValue::Text(match utf8_decode(value) {
				Ok(val) => val,
				Err(e) => return Err(ApeTagItemParseError::from((key, e))),
			}),
			1 => ItemValue::Binary(value),
			2 => ItemValue::Locator(match utf8_decode(value) {
				Ok(val) => val,
				Err(e) => return Err(ApeTagItemParseError::from((key, e))),
			}),
			_ => return Err(ApeTagItemParseError::illegal_item_type(key)),
		};

		let mut item = ApeItem::new(key, parsed_value)?;

		if read_only {
			item.read_only = true;
		}

		Ok(item)
	}

	let mut tag = ApeTag::default();
	let mut remaining_size = header.size;

	for _ in 0..header.item_count {
		if remaining_size < 11 {
			break;
		}

		let value_size = reader.read_u32::<LittleEndian>()?;
		if value_size > remaining_size {
			return Err(SizeMismatchError.into());
		}

		remaining_size -= 4;
		let flags = reader.read_u32::<LittleEndian>()?;

		let mut key = Vec::new();
		let mut key_char = reader.read_u8()?;

		while key_char != 0 {
			key.push(key_char);
			key_char = reader.read_u8()?;
		}

		let key = utf8_decode(key).map_err(ApeTagItemParseError::from)?;

		if APE_PICTURE_TYPES.contains(&&*key) && !parse_options.read_cover_art {
			reader.seek(SeekFrom::Current(i64::from(value_size)))?;
			continue;
		}

		let item = parse_item_with_key(reader, key, flags, value_size)?;
		tag.insert(item);
	}

	// Skip over footer
	reader.seek(SeekFrom::Current(32))?;

	Ok(tag)
}

/// Attempt to read an APE tag from the current position
pub(crate) fn read_ape_tag<R: Read + Seek>(
	reader: &mut R,
	footer: bool,
	parse_options: ParseOptions,
) -> Result<(Option<ApeTag>, Option<ApeHeader>), ApeTagParseError> {
	let mut ape_preamble = [0; 8];
	reader.read_exact(&mut ape_preamble)?;

	let mut ape_tag = None;
	if &ape_preamble == APE_PREAMBLE {
		let ape_header = header::read_ape_header(reader, footer)?;
		if parse_options.read_tags {
			ape_tag = Some(read_ape_tag_with_header(reader, ape_header, parse_options)?);
		}

		return Ok((ape_tag, Some(ape_header)));
	}

	Ok((None, None))
}
