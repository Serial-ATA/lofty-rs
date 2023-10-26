use crate::error::{ErrorKind, LoftyError, Result};
use crate::macros::err;

use std::convert::TryInto;
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

pub(crate) fn decode_text<R>(
	reader: &mut R,
	encoding: TextEncoding,
	terminated: bool,
) -> Result<DecodeTextResult>
where
	R: Read,
{
	let raw_bytes;
	let bytes_read;

	if terminated {
		if let Some(bytes) = read_to_terminator(reader, encoding) {
			let null_terminator_length = match encoding {
				TextEncoding::Latin1 | TextEncoding::UTF8 => 1,
				TextEncoding::UTF16 | TextEncoding::UTF16BE => 2,
			};

			bytes_read = bytes.len() + null_terminator_length;
			raw_bytes = bytes;
		} else {
			return Ok(EMPTY_DECODED_TEXT);
		}
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
	let read_string = match encoding {
		TextEncoding::Latin1 => latin1_decode(&raw_bytes),
		TextEncoding::UTF16 => {
			if raw_bytes.len() < 2 {
				err!(TextDecode("UTF-16 string has an invalid length (< 2)"));
			}

			if raw_bytes.len() % 2 != 0 {
				err!(TextDecode("UTF-16 string has an odd length"));
			}

			match (raw_bytes[0], raw_bytes[1]) {
				(0xFE, 0xFF) => {
					bom = [0xFE, 0xFF];
					utf16_decode_bytes(&raw_bytes[2..], u16::from_be_bytes)?
				},
				(0xFF, 0xFE) => {
					bom = [0xFF, 0xFE];
					utf16_decode_bytes(&raw_bytes[2..], u16::from_le_bytes)?
				},
				_ => err!(TextDecode("UTF-16 string has an invalid byte order mark")),
			}
		},
		TextEncoding::UTF16BE => utf16_decode_bytes(raw_bytes.as_slice(), u16::from_be_bytes)?,
		TextEncoding::UTF8 => utf8_decode(raw_bytes)
			.map_err(|_| LoftyError::new(ErrorKind::TextDecode("Expected a UTF-8 string")))?,
	};

	if read_string.is_empty() {
		return Ok(EMPTY_DECODED_TEXT);
	}

	Ok(DecodeTextResult {
		content: read_string,
		bytes_read,
		bom,
	})
}

pub(crate) fn read_to_terminator<R>(reader: &mut R, encoding: TextEncoding) -> Option<Vec<u8>>
where
	R: Read,
{
	let mut text_bytes = Vec::new();

	match encoding {
		TextEncoding::Latin1 | TextEncoding::UTF8 => {
			while let Ok(byte) = reader.read_u8() {
				if byte == 0 {
					break;
				}

				text_bytes.push(byte)
			}
		},
		TextEncoding::UTF16 | TextEncoding::UTF16BE => {
			while let (Ok(b1), Ok(b2)) = (reader.read_u8(), reader.read_u8()) {
				if b1 == 0 && b2 == 0 {
					break;
				}

				text_bytes.push(b1);
				text_bytes.push(b2)
			}
		},
	}

	if text_bytes.is_empty() {
		return None;
	}

	Some(text_bytes)
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
		.map_while(|c| match c {
			[0, 0] => None,
			_ => Some(endianness(c.try_into().unwrap())), // Infallible
		})
		.collect();

	utf16_decode(&unverified)
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
	use crate::util::text::TextEncoding;
	use std::io::Cursor;

	const TEST_STRING: &str = "l\u{00f8}ft\u{00a5}";

	#[test]
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
			TextEncoding::UTF16,
			false,
		)
		.unwrap();
		let le_utf16_decode = super::decode_text(
			&mut Cursor::new(&[
				0xFF, 0xFE, 0x6C, 0x00, 0xF8, 0x00, 0x66, 0x00, 0x74, 0x00, 0xA5, 0x00, 0x00, 0x00,
			]),
			TextEncoding::UTF16,
			false,
		)
		.unwrap();

		assert_eq!(be_utf16_decode.content, le_utf16_decode.content);
		assert_eq!(be_utf16_decode.bytes_read, le_utf16_decode.bytes_read);
		assert_eq!(be_utf16_decode.content, TEST_STRING.to_string());

		let utf8_decode =
			super::decode_text(&mut TEST_STRING.as_bytes(), TextEncoding::UTF8, false).unwrap();

		assert_eq!(utf8_decode.content, TEST_STRING.to_string());
	}

	#[test]
	fn text_encode() {
		// No BOM
		let utf16_encode = super::utf16_encode(TEST_STRING, u16::to_be_bytes, true, false);

		assert_eq!(
			utf16_encode.as_slice(),
			&[0xFE, 0xFF, 0x00, 0x6C, 0x00, 0xF8, 0x00, 0x66, 0x00, 0x74, 0x00, 0xA5]
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
			&[0xFF, 0xFE, 0x6C, 0x00, 0xF8, 0x00, 0x66, 0x00, 0x74, 0x00, 0xA5, 0x00]
		);
		assert_eq!(
			be_utf16_encode_bom.as_slice(),
			&[0xFE, 0xFF, 0x00, 0x6C, 0x00, 0xF8, 0x00, 0x66, 0x00, 0x74, 0x00, 0xA5]
		);

		let utf8_encode = super::encode_text(TEST_STRING, TextEncoding::UTF8, false);

		assert_eq!(utf8_encode.as_slice(), TEST_STRING.as_bytes());
	}
}
