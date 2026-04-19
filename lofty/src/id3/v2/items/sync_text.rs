use crate::config::WriteOptions;
use crate::error::TooMuchDataError;
use crate::id3::v2::error::FrameParseError;
use crate::id3::v2::frame::error::FrameEncodingError;
use crate::id3::v2::{FrameFlags, FrameHeader, FrameId};
use crate::util::text::{
	DecodeTextResult, TextDecodeOptions, TextEncoding, decode_text,
	utf16_decode_terminated_maybe_bom,
};

use std::borrow::Cow;
use std::io::{Cursor, Seek, Write};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

const FRAME_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("SYLT"));

/// The unit used for [`SynchronizedTextFrame`] timestamps
#[derive(Copy, Clone, PartialEq, Debug, Eq, Hash)]
#[repr(u8)]
pub enum TimestampFormat {
	/// The unit is MPEG frames
	MPEG = 1,
	/// The unit is milliseconds
	MS = 2,
}

/// Invalid timestamp format for a [`SynchronizedTextFrame`]
#[derive(Debug)]
pub struct BadTimestampFormatError;

impl core::fmt::Display for BadTimestampFormatError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.write_str("encountered an invalid timestamp format in a synchronized frame")
	}
}

impl core::error::Error for BadTimestampFormatError {}

impl From<BadTimestampFormatError> for FrameParseError {
	fn from(input: BadTimestampFormatError) -> Self {
		FrameParseError::new(None, Box::new(input))
	}
}

impl TryFrom<u8> for TimestampFormat {
	type Error = BadTimestampFormatError;

	fn try_from(value: u8) -> Result<Self, Self::Error> {
		match value {
			1 => Ok(TimestampFormat::MPEG),
			2 => Ok(TimestampFormat::MS),
			_ => Err(BadTimestampFormatError),
		}
	}
}

/// The type of text stored in a [`SynchronizedTextFrame`]
#[derive(Copy, Clone, PartialEq, Debug, Eq, Hash)]
#[repr(u8)]
pub enum SyncTextContentType {
	/// Other content type
	Other = 0,
	/// Lyrics
	Lyrics = 1,
	/// Text transcription
	TextTranscription = 2,
	/// Movement/part name (e.g. "Adagio")
	PartName = 3,
	/// Events (e.g. "Don Quixote enters the stage")
	Events = 4,
	/// Chord (e.g. "Bb F Fsus")
	Chord = 5,
	/// Trivia/"pop up" information
	Trivia = 6,
	/// URLs to webpages
	WebpageURL = 7,
	/// URLs to images
	ImageURL = 8,
}

/// Invalid content type for a [`SynchronizedTextFrame`]
#[derive(Debug)]
pub struct BadSyncTextContentTypeError;

impl core::fmt::Display for BadSyncTextContentTypeError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.write_str("encountered an invalid synchronized text content type")
	}
}

impl core::error::Error for BadSyncTextContentTypeError {}

impl From<BadSyncTextContentTypeError> for FrameParseError {
	fn from(input: BadSyncTextContentTypeError) -> Self {
		FrameParseError::new(None, Box::new(input))
	}
}

impl TryFrom<u8> for SyncTextContentType {
	type Error = BadSyncTextContentTypeError;

	fn try_from(value: u8) -> Result<Self, Self::Error> {
		match value {
			0 => Ok(Self::Other),
			1 => Ok(Self::Lyrics),
			2 => Ok(Self::TextTranscription),
			3 => Ok(Self::PartName),
			4 => Ok(Self::Events),
			5 => Ok(Self::Chord),
			6 => Ok(Self::Trivia),
			7 => Ok(Self::WebpageURL),
			8 => Ok(Self::ImageURL),
			_ => Err(BadSyncTextContentTypeError),
		}
	}
}

/// Represents an ID3v2 synchronized text frame
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SynchronizedTextFrame<'a> {
	pub(crate) header: FrameHeader<'a>,
	/// The text encoding (description/text)
	pub encoding: TextEncoding,
	/// ISO-639-2 language code (3 bytes)
	pub language: [u8; 3],
	/// The format of the timestamps
	pub timestamp_format: TimestampFormat,
	/// The type of content stored
	pub content_type: SyncTextContentType,
	/// Unique content description
	pub description: Option<String>,
	/// Collection of timestamps and text
	pub content: Vec<(u32, String)>,
}

impl SynchronizedTextFrame<'_> {
	/// Create a new [`SynchronizedTextFrame`]
	pub fn new(
		encoding: TextEncoding,
		language: [u8; 3],
		timestamp_format: TimestampFormat,
		content_type: SyncTextContentType,
		description: Option<String>,
		content: Vec<(u32, String)>,
	) -> Self {
		let header = FrameHeader::new(FRAME_ID, FrameFlags::default());
		Self {
			header,
			encoding,
			language,
			timestamp_format,
			content_type,
			description,
			content,
		}
	}

	/// Get the ID for the frame
	pub fn id(&self) -> FrameId<'_> {
		FRAME_ID
	}

	/// Get the flags for the frame
	pub fn flags(&self) -> FrameFlags {
		self.header.flags
	}

	/// Set the flags for the frame
	pub fn set_flags(&mut self, flags: FrameFlags) {
		self.header.flags = flags;
	}

	/// Read a [`SynchronizedTextFrame`] from a slice
	///
	/// NOTE: This expects the frame header to have already been skipped
	///
	/// # Errors
	///
	/// * Not enough data
	/// * Invalid language
	/// * [`BadTimestampFormatError`]
	/// * [`BadSyncTextContentTypeError`]
	/// * [`TextDecodingError`]
	///
	/// [`TextDecodingError`]: crate::error::TextDecodingError
	#[allow(clippy::missing_panics_doc)] // Infallible
	pub fn parse(data: &[u8], frame_flags: FrameFlags) -> Result<Self, FrameParseError> {
		fn parse_inner<'a>(
			data: &[u8],
			frame_flags: FrameFlags,
		) -> Result<SynchronizedTextFrame<'a>, FrameParseError> {
			if data.len() < 7 {
				return Err(FrameParseError::undersized(FRAME_ID));
			}

			let encoding = TextEncoding::try_from(data[0])?;
			let language: [u8; 3] = data[1..4].try_into().unwrap();
			if language.iter().any(|c| !c.is_ascii_alphabetic()) {
				return Err(FrameParseError::invalid_language(language));
			}
			let timestamp_format = TimestampFormat::try_from(data[4])?;
			let content_type = SyncTextContentType::try_from(data[5])?;

			let mut cursor = Cursor::new(&data[6..]);
			let DecodeTextResult {
				content: description,
				bom,
				..
			} = decode_text(
				&mut cursor,
				TextDecodeOptions::new().encoding(encoding).terminated(true),
			)?;

			// There are 3 possibilities for UTF-16 encoded frames:
			//
			// * The description is the only string with a BOM
			// * The description is empty (has no BOM)
			// * All strings have a BOM
			//
			// To be safe, we change the encoding to the concrete variant determined from the description.
			// Otherwise, we just have to hope that the other fields are encoded properly.
			let endianness: Option<fn([u8; 2]) -> u16> = if encoding == TextEncoding::UTF16 {
				match bom {
					[0xFF, 0xFE] => Some(u16::from_le_bytes),
					[0xFE, 0xFF] => Some(u16::from_be_bytes),
					_ => None,
				}
			} else {
				None
			};

			let mut pos = 0;
			let total = (data.len() - 6) as u64 - cursor.stream_position()?;

			let mut content = Vec::new();

			while pos < total {
				let text;
				if let Some(endianness) = endianness {
					let (decoded, bytes_read) =
						utf16_decode_terminated_maybe_bom(&mut cursor, endianness)?;
					pos += bytes_read as u64;
					text = decoded;
				} else {
					let decoded_text = decode_text(
						&mut cursor,
						TextDecodeOptions::new().encoding(encoding).terminated(true),
					)?;
					pos += decoded_text.bytes_read as u64;

					text = decoded_text.content;
				}

				let time = cursor.read_u32::<BigEndian>()?;
				pos += 4;

				content.push((time, text));
			}

			let header = FrameHeader::new(FRAME_ID, frame_flags);
			Ok(SynchronizedTextFrame {
				header,
				encoding,
				language,
				timestamp_format,
				content_type,
				description: if description.is_empty() {
					None
				} else {
					Some(description)
				},
				content,
			})
		}

		parse_inner(data, frame_flags).map_err(|mut e| {
			e.set_id(FRAME_ID);
			e
		})
	}

	/// Convert a [`SynchronizedTextFrame`] to an ID3v2 SYLT frame byte Vec
	///
	/// NOTE: This does not include the frame header
	///
	/// # Errors
	///
	/// * `content`'s length > [`u32::MAX`]
	/// * `language` is not exactly 3 bytes
	/// * `language` contains invalid characters (Only `'a'..='z'` and `'A'..='Z'` allowed)
	/// * [`WriteOptions::lossy_text_encoding()`] is disabled and the content cannot be encoded in the specified [`TextEncoding`].
	pub fn as_bytes(&self, write_options: WriteOptions) -> Result<Vec<u8>, FrameEncodingError> {
		if !self.language.iter().all(u8::is_ascii_alphabetic) {
			return Err(FrameEncodingError::invalid_language(self.language));
		}

		let mut data = vec![self.encoding as u8];

		data.write_all(&self.language)?;
		data.write_u8(self.timestamp_format as u8)?;
		data.write_u8(self.content_type as u8)?;

		if let Some(description) = &self.description {
			data.write_all(&self.encoding.encode(
				description,
				true,
				write_options.lossy_text_encoding,
			)?)?;
		} else {
			data.write_u8(0)?;
		}

		for (time, text) in &self.content {
			data.write_all(&self.encoding.encode(
				text,
				true,
				write_options.lossy_text_encoding,
			)?)?;
			data.write_u32::<BigEndian>(*time)?;
		}

		if data.len() as u64 > u64::from(u32::MAX) {
			return Err(TooMuchDataError.into());
		}

		Ok(data)
	}
}

#[cfg(test)]
mod tests {
	use crate::config::WriteOptions;
	use crate::id3::v2::{
		FrameFlags, FrameHeader, SyncTextContentType, SynchronizedTextFrame, TimestampFormat,
	};
	use crate::util::text::TextEncoding;

	fn expected(encoding: TextEncoding) -> SynchronizedTextFrame<'static> {
		SynchronizedTextFrame {
			header: FrameHeader::new(super::FRAME_ID, FrameFlags::default()),
			encoding,
			language: *b"eng",
			timestamp_format: TimestampFormat::MS,
			content_type: SyncTextContentType::Lyrics,
			description: Some(String::from("Test Sync Text")),
			content: vec![
				(0, String::from("\nLofty")),
				(10000, String::from("\nIs")),
				(15000, String::from("\nReading")),
				(30000, String::from("\nThis")),
				(1_938_000, String::from("\nCorrectly")),
			],
		}
	}

	#[test_log::test]
	fn sylt_decode() {
		let cont = crate::tag::utils::test_utils::read_path("tests/tags/assets/id3v2/test.sylt");

		let parsed_sylt = SynchronizedTextFrame::parse(&cont, FrameFlags::default()).unwrap();

		assert_eq!(parsed_sylt, expected(TextEncoding::Latin1));
	}

	#[test_log::test]
	fn sylt_encode() {
		let encoded = expected(TextEncoding::Latin1)
			.as_bytes(WriteOptions::default())
			.unwrap();

		let expected_bytes =
			crate::tag::utils::test_utils::read_path("tests/tags/assets/id3v2/test.sylt");

		assert_eq!(encoded, expected_bytes);
	}

	#[test_log::test]
	fn sylt_decode_utf16() {
		let cont =
			crate::tag::utils::test_utils::read_path("tests/tags/assets/id3v2/test_utf16.sylt");

		let parsed_sylt = SynchronizedTextFrame::parse(&cont, FrameFlags::default()).unwrap();

		assert_eq!(parsed_sylt, expected(TextEncoding::UTF16));
	}

	#[test_log::test]
	fn sylt_encode_utf_16() {
		let encoded = expected(TextEncoding::UTF16)
			.as_bytes(WriteOptions::default())
			.unwrap();

		let expected_bytes =
			crate::tag::utils::test_utils::read_path("tests/tags/assets/id3v2/test_utf16.sylt");

		assert_eq!(encoded, expected_bytes);
	}
}
