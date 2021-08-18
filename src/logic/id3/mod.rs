mod constants;
pub(crate) mod v1;
pub(crate) mod v2;

use crate::types::picture::TextEncoding;
use crate::{LoftyError, Result};

use std::io::{Read, Seek, SeekFrom};
use std::ops::Neg;

// https://github.com/polyfloyd/rust-id3/blob/e142ec656bf70a8153f6e5b34a37f26df144c3c1/src/stream/unsynch.rs#L18-L20
pub(crate) fn decode_u32(n: u32) -> u32 {
	n & 0xFF | (n & 0xFF00) >> 1 | (n & 0xFF_0000) >> 2 | (n & 0xFF00_0000) >> 3
}

pub(crate) fn find_lyrics3v2<R>(data: &mut R) -> Result<(bool, u32)>
where
	R: Read + Seek,
{
	let mut exists = false;
	let mut size = 0_u32;

	data.seek(SeekFrom::Current(-15))?;

	let mut lyrics3v2 = [0; 15];
	data.read_exact(&mut lyrics3v2)?;

	if &lyrics3v2[7..] == b"LYRICS200" {
		exists = true;

		let lyrics_size = String::from_utf8(lyrics3v2[..7].to_vec())?;
		let lyrics_size = lyrics_size
			.parse::<u32>()
			.map_err(|_| LoftyError::Ape("Lyrics3v2 tag has an invalid size string"))?;

		size += lyrics_size;

		data.seek(SeekFrom::Current(i64::from(lyrics_size + 15).neg()))?;
	}

	Ok((exists, size))
}

pub(crate) fn encode_text(text: &str, text_encoding: TextEncoding) -> Vec<u8> {
	match text_encoding {
		TextEncoding::Latin1 => text.chars().map(|c| c as u8).collect(),
		TextEncoding::UTF16 => {
			if cfg!(target_endian = "little") {
				utf16_le_encode(text)
			} else {
				utf16_be_encode(text)
			}
		},
		TextEncoding::UTF16BE => utf16_be_encode(text),
		TextEncoding::UTF8 => text.as_bytes().to_vec(),
	}
}

fn utf16_be_encode(text: &str) -> Vec<u8> {
	let mut encoded = Vec::<u8>::new();

	for ch in text.encode_utf16() {
		encoded.extend_from_slice(&ch.to_be_bytes());
	}

	encoded
}

fn utf16_le_encode(text: &str) -> Vec<u8> {
	let mut encoded = Vec::<u8>::new();

	for ch in text.encode_utf16() {
		encoded.extend_from_slice(&ch.to_le_bytes());
	}

	encoded
}
