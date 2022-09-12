use super::RIFFInfoList;
use crate::error::Result;
use crate::iff::chunk::Chunks;
use crate::macros::decode_err;

use std::io::{Read, Seek};

use byteorder::LittleEndian;

pub(in crate::iff::wav) fn parse_riff_info<R>(
	data: &mut R,
	chunks: &mut Chunks<LittleEndian>,
	end: u64,
	tag: &mut RIFFInfoList,
) -> Result<()>
where
	R: Read + Seek,
{
	while data.stream_position()? != end && chunks.next(data).is_ok() {
		let key_str = String::from_utf8(chunks.fourcc.to_vec())
			.map_err(|_| decode_err!(WAV, "Non UTF-8 item key found in RIFF INFO"))?;

		if !verify_key(&key_str) {
			decode_err!(@BAIL WAV, "RIFF INFO item key contains invalid characters");
		}

		tag.items.push((
			key_str,
			chunks
				.read_cstring(data)
				.map_err(|_| decode_err!(WAV, "Failed to read RIFF INFO item value"))?,
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
