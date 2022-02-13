use super::RiffInfoList;
use crate::error::{FileDecodingError, Result};
use crate::iff::chunk::Chunks;
use crate::types::file::FileType;

use std::io::{Read, Seek, SeekFrom};

use byteorder::LittleEndian;

pub(in crate::iff::wav) fn parse_riff_info<R>(
	data: &mut R,
	chunks: &mut Chunks<LittleEndian>,
	end: u64,
	tag: &mut RiffInfoList,
) -> Result<()>
where
	R: Read + Seek,
{
	while data.seek(SeekFrom::Current(0))? != end && chunks.next(data).is_ok() {
		let key_str = String::from_utf8(chunks.fourcc.to_vec()).map_err(|_| {
			FileDecodingError::new(FileType::WAV, "Non UTF-8 item key found in RIFF INFO")
		})?;

		if !verify_key(&key_str) {
			return Err(FileDecodingError::new(
				FileType::WAV,
				"RIFF INFO item key contains invalid characters",
			)
			.into());
		}

		tag.items.push((
			key_str,
			chunks.read_cstring(data).map_err(|_| {
				FileDecodingError::new(FileType::WAV, "Failed to read RIFF INFO item value")
			})?,
		));
	}

	Ok(())
}

pub(super) fn verify_key(key: &str) -> bool {
	key.len() == 4
		&& key
			.chars()
			.all(|c| ('A'..='Z').contains(&c) || ('0'..='9').contains(&c))
}
