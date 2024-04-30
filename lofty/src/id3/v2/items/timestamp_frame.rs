use crate::config::ParsingMode;
use crate::error::{ErrorKind, LoftyError, Result};
use crate::id3::v2::{FrameFlags, FrameHeader, FrameId};
use crate::macros::err;
use crate::tag::items::Timestamp;
use crate::util::text::{decode_text, encode_text, TextDecodeOptions, TextEncoding};

use std::io::Read;

use byteorder::ReadBytesExt;

/// An `ID3v2` timestamp frame
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
#[allow(missing_docs)]
pub struct TimestampFrame<'a> {
	pub(crate) header: FrameHeader<'a>,
	pub encoding: TextEncoding,
	pub timestamp: Timestamp,
}

impl<'a> PartialOrd for TimestampFrame<'a> {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl<'a> Ord for TimestampFrame<'a> {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.timestamp.cmp(&other.timestamp)
	}
}

impl<'a> TimestampFrame<'a> {
	/// Create a new [`TimestampFrame`]
	pub fn new(id: FrameId<'a>, encoding: TextEncoding, timestamp: Timestamp) -> Self {
		let header = FrameHeader::new(id, FrameFlags::default());
		Self {
			header,
			encoding,
			timestamp,
		}
	}

	/// Get the flags for the frame
	pub fn flags(&self) -> FrameFlags {
		self.header.flags
	}

	/// Set the flags for the frame
	pub fn set_flags(&mut self, flags: FrameFlags) {
		self.header.flags = flags;
	}

	/// Read a [`TimestampFrame`]
	///
	/// NOTE: This expects the frame header to have already been skipped
	///
	/// # Errors
	///
	/// * Failure to read from `reader`
	#[allow(clippy::never_loop)]
	pub fn parse<R>(
		reader: &mut R,
		id: FrameId<'a>,
		frame_flags: FrameFlags,
		parse_mode: ParsingMode,
	) -> Result<Option<Self>>
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

		let header = FrameHeader::new(id, frame_flags);
		let mut frame = TimestampFrame {
			header,
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
