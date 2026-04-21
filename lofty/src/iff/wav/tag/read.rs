use super::RiffInfoList;
use crate::config::ParsingMode;
use crate::iff::chunk::{Chunks, valid_fourcc};
use crate::iff::wav::tag::error::RiffInfoListParseError;
use crate::util::text::utf8_decode_str;

use std::io::{Read, Seek};

use byteorder::LittleEndian;

pub(in crate::iff::wav) fn parse_riff_info<R>(
	chunks: &mut Chunks<&mut R, LittleEndian>,
	tag: &mut RiffInfoList,
	parse_mode: ParsingMode,
) -> Result<(), RiffInfoListParseError>
where
	R: Read + Seek,
{
	while let Some(mut chunk) = chunks.next(parse_mode)? {
		let key_str = utf8_decode_str(&chunk.fourcc)?;

		if !valid_fourcc(chunk.fourcc) {
			return Err(RiffInfoListParseError::invalid_fourcc(chunk.fourcc));
		}

		let key = key_str.to_owned();
		let value;
		match chunk.read_string(None) {
			Ok(cstr) => value = cstr,
			Err(e) => {
				if parse_mode == ParsingMode::Strict {
					return Err(e.into());
				}

				// RIFF INFO tags have no standard text encoding, so they will occasionally default
				// to the system encoding, which isn't always UTF-8. In reality, if one item fails
				// they likely all will, but we'll keep trying.
				if e.is_text_decoding_error() {
					continue;
				}

				return Err(e.into());
			},
		}

		tag.items.push((key, value));
	}

	Ok(())
}
