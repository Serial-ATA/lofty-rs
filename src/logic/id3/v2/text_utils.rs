use crate::types::picture::TextEncoding;
use crate::{LoftyError, Result};

use std::convert::TryInto;
use std::io::Read;

use byteorder::ReadBytesExt;

pub(crate) fn decode_text<R>(
	reader: &mut R,
	encoding: TextEncoding,
	terminated: bool,
) -> Result<Option<String>>
where
	R: Read,
{
	let raw_bytes = if terminated {
		read_to_terminator(reader, encoding)
	} else {
		let mut bytes = Vec::new();
		reader.read_to_end(&mut bytes)?;

		(!bytes.is_empty()).then(|| bytes)
	};

	Ok(if let Some(raw_bytes) = raw_bytes {
		Some(match encoding {
			TextEncoding::Latin1 => raw_bytes.iter().map(|c| *c as char).collect::<String>(),
			TextEncoding::UTF16 => {
				if raw_bytes.len() < 2 {
					return Err(LoftyError::TextDecode(
						"UTF-16 string has an invalid length (< 2)",
					));
				}

				match (raw_bytes[0], raw_bytes[1]) {
					(0xFE, 0xFF) => utf16_decode(&raw_bytes[2..], u16::from_be_bytes)?,
					(0xFF, 0xFE) => utf16_decode(&raw_bytes[2..], u16::from_le_bytes)?,
					_ => {
						return Err(LoftyError::TextDecode(
							"UTF-16 string has an invalid byte order mark",
						))
					},
				}
			},
			TextEncoding::UTF16BE => utf16_decode(raw_bytes.as_slice(), u16::from_be_bytes)?,
			TextEncoding::UTF8 => String::from_utf8(raw_bytes)
				.map_err(|_| LoftyError::TextDecode("Expected a UTF-8 string"))?,
		})
	} else {
		None
	})
}

fn utf16_decode(reader: &[u8], endianness: fn([u8; 2]) -> u16) -> Result<String> {
	if reader.len() == 0 || reader.len() % 2 != 0 {
		return Err(LoftyError::TextDecode("UTF-16 string has an odd length"));
	}

	let unverified: Vec<u16> = reader
		.chunks_exact(2)
		.map(|c| endianness(c.try_into().unwrap()))
		.collect();

	String::from_utf16(&unverified)
		.map_err(|_| LoftyError::TextDecode("Given an invalid UTF-16 string"))
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
				if b1 == 0 || b2 == 0 {
					break;
				}

				text_bytes.push(b1);
				text_bytes.push(b2)
			}
		},
	}

	(!text_bytes.is_empty()).then(|| text_bytes)
}

pub(crate) fn encode_text(text: &str, text_encoding: TextEncoding) -> Vec<u8> {
	match text_encoding {
		TextEncoding::Latin1 => text.chars().map(|c| c as u8).collect(),
		TextEncoding::UTF16 => {
			if cfg!(target_endian = "little") {
				utf16_encode(text, u16::to_le_bytes)
			} else {
				utf16_encode(text, u16::to_be_bytes)
			}
		},
		TextEncoding::UTF16BE => utf16_encode(text, u16::to_be_bytes),
		TextEncoding::UTF8 => text.as_bytes().to_vec(),
	}
}

fn utf16_encode(text: &str, endianness: fn(u16) -> [u8; 2]) -> Vec<u8> {
	let mut encoded = Vec::<u8>::new();

	for ch in text.encode_utf16() {
		encoded.extend_from_slice(&endianness(ch));
	}

	encoded
}
