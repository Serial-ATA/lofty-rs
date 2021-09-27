use crate::error::{LoftyError, Result};
use crate::types::item::{ItemKey, ItemValue, TagItem};
use crate::types::tag::{Tag, TagType};

use std::io::{Read, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};

pub(in crate::logic::iff::wav) fn parse_riff_info<R>(
	data: &mut R,
	end: u64,
	tag: &mut Tag,
) -> Result<()>
where
	R: Read + Seek,
{
	while data.seek(SeekFrom::Current(0))? != end {
		let mut key = [0; 4];
		data.read_exact(&mut key)?;

		let key_str = std::str::from_utf8(&key)
			.map_err(|_| LoftyError::Wav("Non UTF-8 key found in RIFF INFO"))?;

		if !key_str.is_ascii() {
			return Err(LoftyError::Wav("Non ascii key found in RIFF INFO"));
		}

		let item_key = ItemKey::from_key(&TagType::RiffInfo, key_str)
			.unwrap_or_else(|| ItemKey::Unknown(key_str.to_string()));

		let size = data.read_u32::<LittleEndian>()?;

		let mut value = vec![0; size as usize];
		data.read_exact(&mut value)?;

		// Values are expected to have an even size, and are padded with a 0 if necessary
		if size % 2 != 0 {
			data.read_u8()?;
		}

		let value_str = std::str::from_utf8(&value)
			.map_err(|_| LoftyError::Wav("Non UTF-8 value found in RIFF INFO"))?;

		tag.insert_item_unchecked(TagItem::new(
			item_key,
			ItemValue::Text(value_str.trim_matches('\0').to_string()),
		));
	}

	Ok(())
}
