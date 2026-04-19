use crate::config::{ParsingMode, WriteOptions};
use crate::id3::v2::error::FrameParseError;
use crate::id3::v2::frame::error::FrameEncodingError;
use crate::id3::v2::{FrameFlags, FrameHeader, FrameId};
use crate::tag::items::Timestamp;
use crate::util::text::{TextDecodeOptions, TextEncoding, decode_text};

use std::io::Read;

use byteorder::ReadBytesExt;

/// An `ID3v2` timestamp frame
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct TimestampFrame<'a> {
	pub(crate) header: FrameHeader<'a>,
	/// The text encoding used for the timestamp
	pub encoding: TextEncoding,
	/// The timestamp value
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
	) -> Result<Option<Self>, FrameParseError>
	where
		R: Read,
	{
		let encoding_byte = match reader.read_u8() {
			Ok(byte) => byte,
			Err(e) => return Err(FrameParseError::new(Some(id.into_owned()), Box::new(e))),
		};

		let encoding = match TextEncoding::try_from(encoding_byte) {
			Ok(encoding) => encoding,
			Err(e) if parse_mode != ParsingMode::Relaxed => {
				return Err(FrameParseError::new(Some(id.into_owned()), Box::new(e)));
			},
			Err(_) => return Ok(None),
		};

		let value = match decode_text(reader, TextDecodeOptions::new().encoding(encoding)) {
			Ok(value) => value.content,
			Err(e) => return Err(FrameParseError::new(Some(id.into_owned()), Box::new(e))),
		};

		let reader = &mut value.as_bytes();

		let result;
		match Timestamp::parse(reader, parse_mode) {
			Ok(timestamp) => result = timestamp,
			Err(e) => {
				if parse_mode != ParsingMode::Relaxed {
					return Err(FrameParseError::new(Some(id.into_owned()), Box::new(e)));
				}
				return Ok(None);
			},
		}

		let Some(timestamp) = result else {
			// Timestamp is empty
			return Ok(None);
		};

		Ok(Some(TimestampFrame {
			header: FrameHeader::new(id, frame_flags),
			encoding,
			timestamp,
		}))
	}

	/// Convert a [`TimestampFrame`] to a byte vec
	///
	/// # Errors
	///
	/// * The timestamp is invalid
	/// * Failure to write to the buffer
	/// * [`WriteOptions::lossy_text_encoding()`] is disabled and the content cannot be encoded in the specified [`TextEncoding`].
	pub fn as_bytes(&self, write_options: WriteOptions) -> Result<Vec<u8>, FrameEncodingError> {
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
