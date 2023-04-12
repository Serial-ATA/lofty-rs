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

pub(crate) fn decode_text<R>(
	reader: &mut R,
	encoding: TextEncoding,
	terminated: bool,
) -> Result<Option<String>>
where
	R: Read,
{
	let raw_bytes;

	if terminated {
		if let Some(bytes) = read_to_terminator(reader, encoding) {
			raw_bytes = bytes
		} else {
			return Ok(None);
		}
	} else {
		let mut bytes = Vec::new();
		reader.read_to_end(&mut bytes)?;

		if bytes.is_empty() {
			return Ok(None);
		}

		raw_bytes = bytes;
	}

	let read_string = match encoding {
		TextEncoding::Latin1 => raw_bytes.iter().map(|c| *c as char).collect::<String>(),
		TextEncoding::UTF16 => {
			if raw_bytes.len() < 2 {
				err!(TextDecode("UTF-16 string has an invalid length (< 2)"));
			}

			if raw_bytes.len() % 2 != 0 {
				err!(TextDecode("UTF-16 string has an odd length"));
			}

			match (raw_bytes[0], raw_bytes[1]) {
				(0xFE, 0xFF) => utf16_decode(&raw_bytes[2..], u16::from_be_bytes)?,
				(0xFF, 0xFE) => utf16_decode(&raw_bytes[2..], u16::from_le_bytes)?,
				_ => err!(TextDecode("UTF-16 string has an invalid byte order mark")),
			}
		},
		TextEncoding::UTF16BE => utf16_decode(raw_bytes.as_slice(), u16::from_be_bytes)?,
		TextEncoding::UTF8 => String::from_utf8(raw_bytes)
			.map_err(|_| LoftyError::new(ErrorKind::TextDecode("Expected a UTF-8 string")))?,
	};

	if !read_string.is_empty() {
		return Ok(Some(read_string));
	}

	Ok(None)
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

	(!text_bytes.is_empty()).then_some(text_bytes)
}

pub(crate) fn utf16_decode(reader: &[u8], endianness: fn([u8; 2]) -> u16) -> Result<String> {
	if reader.is_empty() {
		return Ok(String::new());
	}

	let unverified: Vec<u16> = reader
		.chunks_exact(2)
		.map_while(|c| match c {
			[0, 0] => None,
			_ => Some(endianness(c.try_into().unwrap())), // Infallible
		})
		.collect();

	String::from_utf16(&unverified)
		.map_err(|_| LoftyError::new(ErrorKind::TextDecode("Given an invalid UTF-16 string")))
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
		let utf16_decode = super::utf16_decode(
			&[0x00, 0x6C, 0x00, 0xF8, 0x00, 0x66, 0x00, 0x74, 0x00, 0xA5],
			u16::from_be_bytes,
		)
		.unwrap();

		assert_eq!(utf16_decode, TEST_STRING.to_string());

		// BOM test
		let be_utf16_decode = super::decode_text(
			&mut Cursor::new(&[
				0xFE, 0xFF, 0x00, 0x6C, 0x00, 0xF8, 0x00, 0x66, 0x00, 0x74, 0x00, 0xA5,
			]),
			TextEncoding::UTF16,
			false,
		)
		.unwrap();
		let le_utf16_decode = super::decode_text(
			&mut Cursor::new(&[
				0xFF, 0xFE, 0x6C, 0x00, 0xF8, 0x00, 0x66, 0x00, 0x74, 0x00, 0xA5, 0x00,
			]),
			TextEncoding::UTF16,
			false,
		)
		.unwrap();

		assert_eq!(be_utf16_decode, le_utf16_decode);
		assert_eq!(be_utf16_decode, Some(TEST_STRING.to_string()));

		let utf8_decode =
			super::decode_text(&mut TEST_STRING.as_bytes(), TextEncoding::UTF8, false);

		assert_eq!(utf8_decode.unwrap(), Some(TEST_STRING.to_string()));
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
