use super::RiffInfoList;
use crate::error::Result;
use crate::iff::chunk::Chunks;
use crate::macros::decode_err;
use crate::util::text::utf8_decode_str;

use std::io::{Read, Seek};

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
	while data.stream_position()? != end && chunks.next(data).is_ok() {
		let key_str = utf8_decode_str(&chunks.fourcc)
			.map_err(|_| decode_err!(Wav, "Non UTF-8 item key found in RIFF INFO"))?;

		if !verify_key(key_str) {
			decode_err!(@BAIL Wav, "RIFF INFO item key contains invalid characters");
		}

		tag.items.push((
			key_str.to_owned(),
			chunks
				.read_cstring(data)
				.map_err(|_| decode_err!(Wav, "Failed to read RIFF INFO item value"))?,
		));
	}

	Ok(())
}

pub(super) fn verify_key(key: &str) -> bool {
	key.len() == 4
		&& key
			.chars()
			.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
}
