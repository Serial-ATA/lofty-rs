//! Lyrics3 tag items

use crate::config::WriteOptions;
use crate::error::{TextEncodingError, TooMuchDataError};
use crate::util::text::latin1_encode;

use std::io::Write;

use byteorder::WriteBytesExt;

pub mod v1;
pub mod v2;

/// Write a Lyrics3 Latin-1 string to `writer`
///
/// This enforces the size and character constraints in both Lyrics3 v1 and v2
fn write_latin1_string<W, F, E>(
	context: &str,
	writer: &mut W,
	s: &str,
	max_chars: usize,
	write_options: WriteOptions,
	err: F,
) -> Result<(), E>
where
	W: Write,
	F: FnOnce() -> E,
	E: From<TextEncodingError> + From<TooMuchDataError> + From<std::io::Error>,
{
	let chars = s.chars().count();
	if chars > max_chars {
		if !write_options.implicit_truncation {
			return Err(TooMuchDataError.into());
		}

		log::warn!("{context} too large, truncating (size={chars}, max={max_chars})");
	}

	for b in latin1_encode(s, write_options.lossy_text_encoding).take(max_chars) {
		let mut b = b?;
		if b == 255 {
			if write_options.lossy_text_encoding {
				b = b'?';
			} else {
				return Err(err());
			}
		}

		writer.write_u8(b)?;
	}

	Ok(())
}
