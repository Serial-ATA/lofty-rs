use super::RiffInfoList;
use crate::config::ParsingMode;
use crate::error::{ErrorKind, Result};
use crate::iff::chunk::{Chunks, valid_fourcc};
use crate::macros::decode_err;
use crate::util::text::utf8_decode_str;

use std::io::{Read, Seek};

use crate::TextEncoding;
use byteorder::LittleEndian;

pub(in crate::iff::wav) fn parse_riff_info<R>(
	chunks: &mut Chunks<&mut R, LittleEndian>,
	tag: &mut RiffInfoList,
	parse_mode: ParsingMode,
) -> Result<()>
where
	R: Read + Seek,
{
	while let Some(mut chunk) = chunks.next(parse_mode)? {
		let key_str = utf8_decode_str(&chunk.fourcc)
			.map_err(|_| decode_err!(Wav, "Invalid item key found in RIFF INFO"))?;

		if !valid_fourcc(chunk.fourcc) {
			decode_err!(@BAIL Wav, "RIFF INFO item key contains invalid characters");
		}

		let key = key_str.to_owned();
		let value;
		match chunk.read_cstring() {
			Ok(cstr) => value = cstr,
			Err(e) => {
				if parse_mode == ParsingMode::Strict {
					decode_err!(@BAIL Wav, "Failed to read RIFF INFO item value")
				}

				// RIFF INFO tags have no standard text encoding, so they will occasionally default
				// to the system encoding, which isn't always UTF-8. In reality, if one item fails
				// they likely all will, but we'll keep trying.
				if matches!(e.kind(), ErrorKind::TextDecode(e) if e.encoding() == TextEncoding::UTF8)
				{
					continue;
				}

				return Err(e);
			},
		}

		tag.items.push((key, value));
	}

	Ok(())
}
