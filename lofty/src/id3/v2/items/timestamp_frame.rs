use crate::config::{ParsingMode, WriteOptions};
use crate::error::Result;
use crate::id3::v2::{FrameFlags, FrameHeader, FrameId};
use crate::macros::err;
use crate::tag::items::Timestamp;
use crate::util::text::{TextDecodeOptions, TextEncoding, decode_text};

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

impl PartialOrd for TimestampFrame<'_> {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for TimestampFrame<'_> {
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

	/// Get the ID for the frame
	pub fn id(&self) -> &FrameId<'_> {
		&self.header.id
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
			if parse_mode != ParsingMode::Relaxed {
				err!(TextDecode("Found invalid encoding"))
			}
			return Ok(None);
		};

		let value = decode_text(reader, TextDecodeOptions::new().encoding(encoding))?.content;
		if !value.is_ascii() {
			if parse_mode == ParsingMode::Strict {
				err!(BadTimestamp("Timestamp contains non-ASCII characters"))
			}
			return Ok(None);
		}

		let header = FrameHeader::new(id, frame_flags);
		let mut frame = TimestampFrame {
			header,
			encoding,
			timestamp: Timestamp::default(),
		};

		let reader = &mut value.as_bytes();

		let result;
		match Timestamp::parse(reader, parse_mode) {
			Ok(timestamp) => result = timestamp,
			Err(e) => {
				if parse_mode != ParsingMode::Relaxed {
					return Err(e);
				}
				return Ok(None);
			},
		}

		let Some(timestamp) = result else {
			// Timestamp is empty
			return Ok(None);
		};

		frame.timestamp = timestamp;
		Ok(Some(frame))
	}

	/// Convert a [`TimestampFrame`] to a byte vec
	///
	/// # Errors
	///
	/// * The timestamp is invalid
	/// * Failure to write to the buffer
	/// * [`WriteOptions::lossy_text_encoding()`] is disabled and the content cannot be encoded in the specified [`TextEncoding`].
	pub fn as_bytes(&self, write_options: WriteOptions) -> Result<Vec<u8>> {
		let mut encoding = self.encoding;
		if write_options.use_id3v23 {
			encoding = encoding.to_id3v23();
		}

		self.timestamp.verify()?;

		let mut encoded_text = encoding.encode(
			&self.timestamp.to_string(),
			false,
			write_options.lossy_text_encoding,
		)?;
		encoded_text.insert(0, encoding as u8);

		Ok(encoded_text)
	}
}

impl TimestampFrame<'static> {
	pub(crate) fn downgrade(&self) -> TimestampFrame<'_> {
		TimestampFrame {
			header: self.header.downgrade(),
			encoding: self.encoding,
			timestamp: self.timestamp,
		}
	}
}
