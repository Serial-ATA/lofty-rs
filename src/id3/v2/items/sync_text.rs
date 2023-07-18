use crate::error::{ErrorKind, Id3v2Error, Id3v2ErrorKind, LoftyError, Result};
use crate::macros::err;
use crate::util::text::{decode_text, encode_text, read_to_terminator, utf16_decode, TextEncoding};

use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

/// The unit used for [`SynchronizedText`] timestamps
#[derive(Copy, Clone, PartialEq, Debug, Eq, Hash)]
#[repr(u8)]
pub enum TimestampFormat {
	/// The unit is MPEG frames
	MPEG = 1,
	/// The unit is milliseconds
	MS = 2,
}

impl TimestampFormat {
	/// Get a `TimestampFormat` from a u8, must be 1-2 inclusive
	pub fn from_u8(byte: u8) -> Option<Self> {
		match byte {
			1 => Some(Self::MPEG),
			2 => Some(Self::MS),
			_ => None,
		}
	}
}

/// The type of text stored in a [`SynchronizedText`]
#[derive(Copy, Clone, PartialEq, Debug, Eq, Hash)]
#[repr(u8)]
#[allow(missing_docs)]
pub enum SyncTextContentType {
	Other = 0,
	Lyrics = 1,
	TextTranscription = 2,
	PartName = 3,
	Events = 4,
	Chord = 5,
	Trivia = 6,
	WebpageURL = 7,
	ImageURL = 8,
}

impl SyncTextContentType {
	/// Get a `SyncTextContentType` from a u8, must be 0-8 inclusive
	pub fn from_u8(byte: u8) -> Option<Self> {
		match byte {
			0 => Some(Self::Other),
			1 => Some(Self::Lyrics),
			2 => Some(Self::TextTranscription),
			3 => Some(Self::PartName),
			4 => Some(Self::Events),
			5 => Some(Self::Chord),
			6 => Some(Self::Trivia),
			7 => Some(Self::WebpageURL),
			8 => Some(Self::ImageURL),
			_ => None,
		}
	}
}

/// Represents an ID3v2 synchronized text frame
#[derive(PartialEq, Clone, Debug, Eq, Hash)]
pub struct SynchronizedText {
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

impl SynchronizedText {
	/// Read a [`SynchronizedText`] from a slice
	///
	/// NOTE: This expects the frame header to have already been skipped
	///
	/// # Errors
	///
	/// This function will return [`BadSyncText`][Id3v2ErrorKind::BadSyncText] if at any point it's unable to parse the data
	#[allow(clippy::missing_panics_doc)] // Infallible
	pub fn parse(data: &[u8]) -> Result<Self> {
		if data.len() < 7 {
			return Err(Id3v2Error::new(Id3v2ErrorKind::BadFrameLength).into());
		}

		let encoding = TextEncoding::from_u8(data[0])
			.ok_or_else(|| LoftyError::new(ErrorKind::TextDecode("Found invalid encoding")))?;
		let language: [u8; 3] = data[1..4].try_into().unwrap();
		if language.iter().any(|c| !c.is_ascii_alphabetic()) {
			return Err(Id3v2Error::new(Id3v2ErrorKind::BadSyncText).into());
		}
		let timestamp_format = TimestampFormat::from_u8(data[4])
			.ok_or_else(|| Id3v2Error::new(Id3v2ErrorKind::BadTimestampFormat))?;
		let content_type = SyncTextContentType::from_u8(data[5])
			.ok_or_else(|| Id3v2Error::new(Id3v2ErrorKind::BadSyncText))?;

		let mut cursor = Cursor::new(&data[6..]);
		let description = crate::util::text::decode_text(&mut cursor, encoding, true)
			.map_err(|_| Id3v2Error::new(Id3v2ErrorKind::BadSyncText))?
			.text_or_none();

		let mut endianness: fn([u8; 2]) -> u16 = u16::from_le_bytes;

		// It's possible for the description to be the only string with a BOM
		// To be safe, we change the encoding to the concrete variant determined from the description
		if encoding == TextEncoding::UTF16 {
			endianness = match cursor.get_ref()[..=1] {
				[0xFF, 0xFE] => u16::from_le_bytes,
				[0xFE, 0xFF] => u16::from_be_bytes,
				// Since the description was already read, we can assume the BOM was valid
				_ => unreachable!(),
			};
		}

		let mut pos = 0;
		let total = (data.len() - 6) as u64 - cursor.stream_position()?;

		let mut content = Vec::new();

		while pos < total {
			let text = (|| -> Result<String> {
				if encoding == TextEncoding::UTF16 {
					// Check for a BOM
					let mut bom = [0; 2];
					cursor
						.read_exact(&mut bom)
						.map_err(|_| Id3v2Error::new(Id3v2ErrorKind::BadSyncText))?;

					cursor.seek(SeekFrom::Current(-2))?;

					// Encountered text that doesn't include a BOM
					if bom != [0xFF, 0xFE] && bom != [0xFE, 0xFF] {
						if let Some(raw_text) = read_to_terminator(&mut cursor, TextEncoding::UTF16)
						{
							// text + null terminator
							pos += (raw_text.len() + 2) as u64;

							return utf16_decode(&raw_text, endianness)
								.map_err(|_| Id3v2Error::new(Id3v2ErrorKind::BadSyncText).into());
						}

						return Ok(String::new());
					}
				}

				let decoded_text = decode_text(&mut cursor, encoding, true)
					.map_err(|_| Id3v2Error::new(Id3v2ErrorKind::BadSyncText))?;
				pos += decoded_text.bytes_read as u64;

				Ok(decoded_text.content)
			})()?;

			let time = cursor
				.read_u32::<BigEndian>()
				.map_err(|_| Id3v2Error::new(Id3v2ErrorKind::BadSyncText))?;
			pos += 4;

			content.push((time, text));
		}

		Ok(Self {
			encoding,
			language,
			timestamp_format,
			content_type,
			description,
			content,
		})
	}

	/// Convert a [`SynchronizedText`] to an ID3v2 SYLT frame byte Vec
	///
	/// NOTE: This does not include the frame header
	///
	/// # Errors
	///
	/// * `content`'s length > [`u32::MAX`]
	/// * `language` is not exactly 3 bytes
	/// * `language` contains invalid characters (Only `'a'..='z'` and `'A'..='Z'` allowed)
	pub fn as_bytes(&self) -> Result<Vec<u8>> {
		let mut data = vec![self.encoding as u8];

		if self.language.len() == 3 && self.language.iter().all(u8::is_ascii_alphabetic) {
			data.write_all(&self.language)?;
			data.write_u8(self.timestamp_format as u8)?;
			data.write_u8(self.content_type as u8)?;

			if let Some(description) = &self.description {
				data.write_all(&encode_text(description, self.encoding, true))?;
			} else {
				data.write_u8(0)?;
			}

			for (time, ref text) in &self.content {
				data.write_all(&encode_text(text, self.encoding, true))?;
				data.write_u32::<BigEndian>(*time)?;
			}

			if data.len() as u64 > u64::from(u32::MAX) {
				err!(TooMuchData);
			}

			return Ok(data);
		}

		Err(Id3v2Error::new(Id3v2ErrorKind::BadSyncText).into())
	}
}

#[cfg(test)]
mod tests {
	use crate::id3::v2::{SyncTextContentType, SynchronizedText, TimestampFormat};
	use crate::util::text::TextEncoding;

	fn expected(encoding: TextEncoding) -> SynchronizedText {
		SynchronizedText {
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

	#[test]
	fn sylt_decode() {
		let cont = crate::tag::utils::test_utils::read_path("tests/tags/assets/id3v2/test.sylt");

		let parsed_sylt = SynchronizedText::parse(&cont).unwrap();

		assert_eq!(parsed_sylt, expected(TextEncoding::Latin1));
	}

	#[test]
	fn sylt_encode() {
		let encoded = expected(TextEncoding::Latin1).as_bytes().unwrap();

		let expected_bytes =
			crate::tag::utils::test_utils::read_path("tests/tags/assets/id3v2/test.sylt");

		assert_eq!(encoded, expected_bytes);
	}

	#[test]
	fn sylt_decode_utf16() {
		let cont =
			crate::tag::utils::test_utils::read_path("tests/tags/assets/id3v2/test_utf16.sylt");

		let parsed_sylt = SynchronizedText::parse(&cont).unwrap();

		assert_eq!(parsed_sylt, expected(TextEncoding::UTF16));
	}

	#[test]
	fn sylt_encode_utf_16() {
		let encoded = expected(TextEncoding::UTF16).as_bytes().unwrap();

		let expected_bytes =
			crate::tag::utils::test_utils::read_path("tests/tags/assets/id3v2/test_utf16.sylt");

		assert_eq!(encoded, expected_bytes);
	}
}
