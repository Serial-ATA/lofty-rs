use crate::error::{ErrorKind, LoftyError, Result};
use crate::macros::err;

use std::io::Read;

use byteorder::ReadBytesExt;

/// The text encoding for use in ID3v2 frames
#[derive(Debug, Clone, Eq, PartialEq, Copy, Hash)]
#[repr(u8)]
pub enum TextEncoding {
	/// ISO-8859-1
	Latin1 = 0,
	/// UTF-16 with a byte order mark
	UTF16 = 1,
	/// UTF-16 big endian
	UTF16BE = 2,
	/// UTF-8
	UTF8 = 3,
}

impl TextEncoding {
	/// Get a `TextEncoding` from a u8, must be 0-3 inclusive
	pub fn from_u8(byte: u8) -> Option<Self> {
		match byte {
			0 => Some(Self::Latin1),
			1 => Some(Self::UTF16),
			2 => Some(Self::UTF16BE),
			3 => Some(Self::UTF8),
			_ => None,
		}
	}

	pub(crate) fn verify_latin1(text: &str) -> bool {
		text.chars().all(|c| c as u32 <= 255)
	}

	/// ID3v2.4 introduced two new text encodings.
	///
	/// When writing ID3v2.3, we just substitute with UTF-16.
	pub(crate) fn to_id3v23(self) -> Self {
		match self {
			Self::UTF8 | Self::UTF16BE => {
				log::warn!(
					"Text encoding {:?} is not supported in ID3v2.3, substituting with UTF-16",
					self
				);
				Self::UTF16
			},
			_ => self,
		}
	}
}

#[derive(Eq, PartialEq, Debug)]
pub(crate) struct DecodeTextResult {
	pub(crate) content: String,
	pub(crate) bytes_read: usize,
	pub(crate) bom: [u8; 2],
}

impl DecodeTextResult {
	pub(crate) fn text_or_none(self) -> Option<String> {
		if self.content.is_empty() {
			return None;
		}

		Some(self.content)
	}
}

const EMPTY_DECODED_TEXT: DecodeTextResult = DecodeTextResult {
	content: String::new(),
	bytes_read: 0,
	bom: [0, 0],
};

/// Specify how to decode the provided text
///
/// By default, this will:
///
/// * Use [`TextEncoding::UTF8`] as the encoding
/// * Not expect the text to be null terminated
/// * Have no byte order mark
#[derive(Copy, Clone, Debug)]
pub(crate) struct TextDecodeOptions {
	pub encoding: TextEncoding,
	pub terminated: bool,
	pub bom: [u8; 2],
}

impl TextDecodeOptions {
	pub(crate) fn new() -> Self {
		Self::default()
	}

	pub(crate) fn encoding(mut self, encoding: TextEncoding) -> Self {
		self.encoding = encoding;
		self
	}

	pub(crate) fn terminated(mut self, terminated: bool) -> Self {
		self.terminated = terminated;
		self
	}

	pub(crate) fn bom(mut self, bom: [u8; 2]) -> Self {
		self.bom = bom;
		self
	}
}

impl Default for TextDecodeOptions {
	fn default() -> Self {
		Self {
			encoding: TextEncoding::UTF8,
			terminated: false,
			bom: [0, 0],
		}
	}
}

pub(crate) fn decode_text<R>(reader: &mut R, options: TextDecodeOptions) -> Result<DecodeTextResult>
where
	R: Read,
{
	let raw_bytes;
	let bytes_read;

	if options.terminated {
		let (bytes, terminator_len) = read_to_terminator(reader, options.encoding);

		if bytes.is_empty() {
			return Ok(EMPTY_DECODED_TEXT);
		}

		bytes_read = bytes.len() + terminator_len;
		raw_bytes = bytes;
	} else {
		let mut bytes = Vec::new();
		reader.read_to_end(&mut bytes)?;

		if bytes.is_empty() {
			return Ok(EMPTY_DECODED_TEXT);
		}

		bytes_read = bytes.len();
		raw_bytes = bytes;
	}

	let mut bom = [0, 0];
	let read_string = match options.encoding {
		TextEncoding::Latin1 => latin1_decode(&raw_bytes),
		TextEncoding::UTF16 => {
			if raw_bytes.len() < 2 {
				err!(TextDecode("UTF-16 string has an invalid length (< 2)"));
			}

			if raw_bytes.len() % 2 != 0 {
				err!(TextDecode("UTF-16 string has an odd length"));
			}

			if options.bom == [0, 0] {
				bom = [raw_bytes[0], raw_bytes[1]];
			} else {
				bom = options.bom;
			}

			match bom {
				[0xFE, 0xFF] => utf16_decode_bytes(&raw_bytes[2..], u16::from_be_bytes)?,
				[0xFF, 0xFE] => utf16_decode_bytes(&raw_bytes[2..], u16::from_le_bytes)?,
				_ => err!(TextDecode("UTF-16 string has an invalid byte order mark")),
			}
		},
		TextEncoding::UTF16BE => utf16_decode_bytes(raw_bytes.as_slice(), u16::from_be_bytes)?,
		TextEncoding::UTF8 => utf8_decode(raw_bytes)
			.map_err(|_| LoftyError::new(ErrorKind::TextDecode("Expected a UTF-8 string")))?,
	};

	Ok(DecodeTextResult {
		content: read_string,
		bytes_read,
		bom,
	})
}

pub(crate) fn read_to_terminator<R>(reader: &mut R, encoding: TextEncoding) -> (Vec<u8>, usize)
where
	R: Read,
{
	let mut text_bytes = Vec::new();
	let mut terminator_len = 0;

	match encoding {
		TextEncoding::Latin1 | TextEncoding::UTF8 => {
			while let Ok(byte) = reader.read_u8() {
				if byte == 0 {
					terminator_len = 1;
					break;
				}

				text_bytes.push(byte)
			}
		},
		TextEncoding::UTF16 | TextEncoding::UTF16BE => {
			while let (Ok(b1), Ok(b2)) = (reader.read_u8(), reader.read_u8()) {
				if b1 == 0 && b2 == 0 {
					terminator_len = 2;
					break;
				}

				text_bytes.push(b1);
				text_bytes.push(b2)
			}
		},
	}

	(text_bytes, terminator_len)
}

pub(crate) fn latin1_decode(bytes: &[u8]) -> String {
	let mut text = bytes.iter().map(|c| *c as char).collect::<String>();
	trim_end_nulls(&mut text);
	text
}

pub(crate) fn utf8_decode(bytes: Vec<u8>) -> Result<String> {
	String::from_utf8(bytes)
		.map(|mut text| {
			trim_end_nulls(&mut text);
			text
		})
		.map_err(Into::into)
}

pub(crate) fn utf8_decode_str(bytes: &[u8]) -> Result<&str> {
	std::str::from_utf8(bytes)
		.map(trim_end_nulls_str)
		.map_err(Into::into)
}

pub(crate) fn utf16_decode(words: &[u16]) -> Result<String> {
	String::from_utf16(words)
		.map(|mut text| {
			trim_end_nulls(&mut text);
			text
		})
		.map_err(|_| LoftyError::new(ErrorKind::TextDecode("Given an invalid UTF-16 string")))
}

pub(crate) fn utf16_decode_bytes(bytes: &[u8], endianness: fn([u8; 2]) -> u16) -> Result<String> {
	if bytes.is_empty() {
		return Ok(String::new());
	}

	let unverified: Vec<u16> = bytes
		.chunks_exact(2)
		// In ID3v2, it is possible to have multiple UTF-16 strings separated by null.
		// This also makes it possible for us to encounter multiple BOMs in a single string.
		// We must filter them out.
		.filter_map(|c| match c {
			[0xFF, 0xFE] | [0xFE, 0xFF] => None,
			_ => Some(endianness(c.try_into().unwrap())), // Infallible
		})
		.collect();

	utf16_decode(&unverified)
}

// TODO: Can probably just be merged into an option on `TextDecodeOptions`
/// Read a null-terminated UTF-16 string that may or may not have a BOM
///
/// This is needed for ID3v2, as some encoders will encode *only* the first string in a frame with a BOM,
/// and the others are assumed to have the same byte order.
///
/// This is seen in frames like SYLT, COMM, and USLT, where the description will be the only string
/// with a BOM.
///
/// If no BOM is present, the string will be decoded using `endianness`.
pub(crate) fn utf16_decode_terminated_maybe_bom<R>(
	reader: &mut R,
	endianness: fn([u8; 2]) -> u16,
) -> Result<(String, usize)>
where
	R: Read,
{
	let (raw_text, terminator_len) = read_to_terminator(reader, TextEncoding::UTF16);

	let bytes_read = raw_text.len() + terminator_len;
	let decoded;
	match &*raw_text {
		[0xFF, 0xFE, ..] => decoded = utf16_decode_bytes(&raw_text[2..], u16::from_le_bytes),
		[0xFE, 0xFF, ..] => decoded = utf16_decode_bytes(&raw_text[2..], u16::from_be_bytes),
		_ => decoded = utf16_decode_bytes(&raw_text, endianness),
	}

	decoded.map(|d| (d, bytes_read))
}

pub(crate) fn encode_text(text: &str, text_encoding: TextEncoding, terminated: bool) -> Vec<u8> {
	match text_encoding {
		TextEncoding::Latin1 => {
			let mut out = text.chars().map(|c| c as u8).collect::<Vec<u8>>();

			if terminated {
				out.push(0)
			}

			out
		},
		TextEncoding::UTF16 => utf16_encode(text, u16::to_ne_bytes, true, terminated),
		TextEncoding::UTF16BE => utf16_encode(text, u16::to_be_bytes, false, terminated),
		TextEncoding::UTF8 => {
			let mut out = text.as_bytes().to_vec();

			if terminated {
				out.push(0);
			}

			out
		},
	}
}

pub(crate) fn trim_end_nulls(text: &mut String) {
	if text.ends_with('\0') {
		let new_len = text.trim_end_matches('\0').len();
		text.truncate(new_len);
	}
}

pub(crate) fn trim_end_nulls_str(text: &str) -> &str {
	text.trim_end_matches('\0')
}

fn utf16_encode(
	text: &str,
	endianness: fn(u16) -> [u8; 2],
	bom: bool,
	terminated: bool,
) -> Vec<u8> {
	let mut encoded = Vec::<u8>::new();

	if bom {
		encoded.extend_from_slice(&endianness(0xFEFF_u16));
	}

	for ch in text.encode_utf16() {
		encoded.extend_from_slice(&endianness(ch));
	}

	if terminated {
		encoded.extend_from_slice(&[0, 0]);
	}

	encoded
}

#[cfg(test)]
mod tests {
	use crate::util::text::{TextDecodeOptions, TextEncoding};
	use std::io::Cursor;

	const TEST_STRING: &str = "l\u{00f8}ft\u{00a5}";

	#[test_log::test]
	fn text_decode() {
		// No BOM
		let utf16_decode = super::utf16_decode_bytes(
			&[
				0x00, 0x6C, 0x00, 0xF8, 0x00, 0x66, 0x00, 0x74, 0x00, 0xA5, 0x00, 0x00,
			],
			u16::from_be_bytes,
		)
		.unwrap();

		assert_eq!(utf16_decode, TEST_STRING.to_string());

		// BOM test
		let be_utf16_decode = super::decode_text(
			&mut Cursor::new(&[
				0xFE, 0xFF, 0x00, 0x6C, 0x00, 0xF8, 0x00, 0x66, 0x00, 0x74, 0x00, 0xA5, 0x00, 0x00,
			]),
			TextDecodeOptions::new().encoding(TextEncoding::UTF16),
		)
		.unwrap();
		let le_utf16_decode = super::decode_text(
			&mut Cursor::new(&[
				0xFF, 0xFE, 0x6C, 0x00, 0xF8, 0x00, 0x66, 0x00, 0x74, 0x00, 0xA5, 0x00, 0x00, 0x00,
			]),
			TextDecodeOptions::new().encoding(TextEncoding::UTF16),
		)
		.unwrap();

		assert_eq!(be_utf16_decode.content, le_utf16_decode.content);
		assert_eq!(be_utf16_decode.bytes_read, le_utf16_decode.bytes_read);
		assert_eq!(be_utf16_decode.content, TEST_STRING.to_string());

		let utf8_decode = super::decode_text(
			&mut TEST_STRING.as_bytes(),
			TextDecodeOptions::new().encoding(TextEncoding::UTF8),
		)
		.unwrap();

		assert_eq!(utf8_decode.content, TEST_STRING.to_string());
	}

	#[test_log::test]
	fn text_encode() {
		// No BOM
		let utf16_encode = super::utf16_encode(TEST_STRING, u16::to_be_bytes, true, false);

		assert_eq!(
			utf16_encode.as_slice(),
			&[
				0xFE, 0xFF, 0x00, 0x6C, 0x00, 0xF8, 0x00, 0x66, 0x00, 0x74, 0x00, 0xA5
			]
		);

		// BOM test
		let be_utf16_encode = super::encode_text(TEST_STRING, TextEncoding::UTF16BE, false);
		let le_utf16_encode = super::utf16_encode(TEST_STRING, u16::to_le_bytes, true, false);
		let be_utf16_encode_bom = super::utf16_encode(TEST_STRING, u16::to_be_bytes, true, false);

		assert_ne!(be_utf16_encode.as_slice(), le_utf16_encode.as_slice());
		// TextEncoding::UTF16BE has no BOM
		assert_eq!(
			be_utf16_encode.as_slice(),
			&[0x00, 0x6C, 0x00, 0xF8, 0x00, 0x66, 0x00, 0x74, 0x00, 0xA5]
		);
		assert_eq!(
			le_utf16_encode.as_slice(),
			&[
				0xFF, 0xFE, 0x6C, 0x00, 0xF8, 0x00, 0x66, 0x00, 0x74, 0x00, 0xA5, 0x00
			]
		);
		assert_eq!(
			be_utf16_encode_bom.as_slice(),
			&[
				0xFE, 0xFF, 0x00, 0x6C, 0x00, 0xF8, 0x00, 0x66, 0x00, 0x74, 0x00, 0xA5
			]
		);

		let utf8_encode = super::encode_text(TEST_STRING, TextEncoding::UTF8, false);

		assert_eq!(utf8_encode.as_slice(), TEST_STRING.as_bytes());
	}
}
