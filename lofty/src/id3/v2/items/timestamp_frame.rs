use crate::config::ParsingMode;
use crate::error::{ErrorKind, LoftyError, Result};
use crate::macros::err;
use crate::tag::items::Timestamp;
use crate::util::text::{decode_text, encode_text, TextDecodeOptions, TextEncoding};

use std::io::Read;

use byteorder::ReadBytesExt;

/// An `ID3v2` timestamp frame
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[allow(missing_docs)]
pub struct TimestampFrame {
	pub encoding: TextEncoding,
	pub timestamp: Timestamp,
}

impl PartialOrd for TimestampFrame {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for TimestampFrame {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.timestamp.cmp(&other.timestamp)
	}
}

impl Default for TimestampFrame {
	fn default() -> Self {
		Self {
			encoding: TextEncoding::UTF8,
			timestamp: Timestamp::default(),
		}
	}
}

impl TimestampFrame {
	/// Read a [`TimestampFrame`]
	///
	/// NOTE: This expects the frame header to have already been skipped
	///
	/// # Errors
	///
	/// * Failure to read from `reader`
	#[allow(clippy::never_loop)]
	pub fn parse<R>(reader: &mut R, parse_mode: ParsingMode) -> Result<Option<Self>>
	where
		R: Read,
	{
		let Ok(encoding_byte) = reader.read_u8() else {
			return Ok(None);
		};
		let Some(encoding) = TextEncoding::from_u8(encoding_byte) else {
			return Err(LoftyError::new(ErrorKind::TextDecode(
				"Found invalid encoding",
			)));
		};

		let value = decode_text(reader, TextDecodeOptions::new().encoding(encoding))?.content;
		if !value.is_ascii() {
			err!(BadTimestamp("Timestamp contains non-ASCII characters"))
		}

		let mut frame = TimestampFrame {
			encoding,
			timestamp: Timestamp::default(),
		};

		let reader = &mut value.as_bytes();

		frame.timestamp = Timestamp::parse(reader, parse_mode)?;
		Ok(Some(frame))
	}

	/// Convert an [`TimestampFrame`] to a byte vec
	///
	/// # Errors
	///
	/// * The timestamp is invalid
	/// * Failure to write to the buffer
	pub fn as_bytes(&self) -> Result<Vec<u8>> {
		self.timestamp.verify()?;

		let mut encoded_text = encode_text(&self.timestamp.to_string(), self.encoding, false);
		encoded_text.insert(0, self.encoding as u8);

		Ok(encoded_text)
	}
}
