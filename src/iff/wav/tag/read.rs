use super::RiffInfoList;
use crate::error::{LoftyError, Result};
use crate::iff::chunk::Chunks;

use std::io::{Read, Seek, SeekFrom};

use byteorder::LittleEndian;

pub(in crate::iff::wav) fn parse_riff_info<R>(
	data: &mut R,
	end: u64,
	tag: &mut RiffInfoList,
) -> Result<()>
where
	R: Read + Seek,
{
	let mut chunks = Chunks::<LittleEndian>::new();

	while data.seek(SeekFrom::Current(0))? != end && chunks.next(data).is_ok() {
		let key_str = String::from_utf8(chunks.fourcc.to_vec())
			.map_err(|_| LoftyError::Wav("Non UTF-8 key found in RIFF INFO"))?;

		if !key_str.is_ascii() {
			return Err(LoftyError::Wav("Non-ascii key found in RIFF INFO"));
		}

		tag.items.push((
			key_str.to_string(),
			chunks
				.read_cstring(data)
				.map_err(|_| LoftyError::Wav("Failed to read the chunk value"))?,
		));
	}

	Ok(())
}
