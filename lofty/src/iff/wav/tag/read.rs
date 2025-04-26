use super::RiffInfoList;
use crate::config::ParsingMode;
use crate::error::{ErrorKind, Result};
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
	parse_mode: ParsingMode,
) -> Result<()>
where
	R: Read + Seek,
{
	while data.stream_position()? != end && matches!(chunks.next(data), Ok(true)) {
		let key_str = utf8_decode_str(&chunks.fourcc)
			.map_err(|_| decode_err!(Wav, "Invalid item key found in RIFF INFO"))?;

		if !verify_key(key_str) {
			decode_err!(@BAIL Wav, "RIFF INFO item key contains invalid characters");
		}

		let key = key_str.to_owned();
		let value;
		match chunks.read_cstring(data) {
			Ok(cstr) => value = cstr,
			Err(e) => {
				if parse_mode == ParsingMode::Strict {
					decode_err!(@BAIL Wav, "Failed to read RIFF INFO item value")
				}

				// RIFF INFO tags have no standard text encoding, so they will occasionally default
				// to the system encoding, which isn't always UTF-8. In reality, if one item fails
				// they likely all will, but we'll keep trying.
				if matches!(e.kind(), ErrorKind::StringFromUtf8(_)) {
					continue;
				}

				return Err(e);
			},
		}

		tag.items.push((key, value));
	}

	Ok(())
}

pub(super) fn verify_key(key: &str) -> bool {
	key.len() == 4
		&& key
			.chars()
			.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
}
