use crate::error::{ID3v2Error, ID3v2ErrorKind, LoftyError, Result};
use crate::id3::v2::frame::FrameValue;
use crate::id3::v2::items::{
	ExtendedTextFrame, LanguageFrame, Popularimeter, UniqueFileIdentifierFrame,
};
use crate::id3::v2::ID3v2Version;
use crate::macros::err;
use crate::picture::Picture;
use crate::util::text::{decode_text, read_to_terminator, utf16_decode, TextEncoding};

use std::io::{Cursor, Read};

use byteorder::ReadBytesExt;

#[rustfmt::skip]
pub(super) fn parse_content(
	content: &mut &[u8],
	id: &str,
	version: ID3v2Version,
) -> Result<Option<FrameValue>> {
	Ok(match id {
		// The ID was previously upgraded, but the content remains unchanged, so version is necessary
		"APIC" => {
			let (picture, encoding) = Picture::from_apic_bytes(content, version)?;

			Some(FrameValue::Picture { encoding, picture })
		},
		"TXXX" => parse_user_defined(content, false, version)?,
		"WXXX" => parse_user_defined(content, true, version)?,
		"COMM" | "USLT" => parse_text_language(content, id, version)?,
		"UFID" => UniqueFileIdentifierFrame::decode_bytes(content)?.map(FrameValue::UniqueFileIdentifier),
		_ if id.starts_with('T') => parse_text(content, version)?,
		// Apple proprietary frames
		// WFED (Podcast URL), GRP1 (Grouping), MVNM (Movement Name), MVIN (Movement Number)
		"WFED" | "GRP1" | "MVNM" | "MVIN" => parse_text(content, version)?,
		_ if id.starts_with('W') => parse_link(content)?,
		"POPM" => Some(FrameValue::Popularimeter(Popularimeter::from_bytes(content)?)),
		// SYLT, GEOB, and any unknown frames
		_ => Some(FrameValue::Binary(content.to_vec())),
	})
}

// There are 2 possibilities for the frame's content: text or link.
fn parse_user_defined(
	mut content: &mut &[u8],
	link: bool,
	version: ID3v2Version,
) -> Result<Option<FrameValue>> {
	if content.len() < 2 {
		return Ok(None);
	}

	let encoding = verify_encoding(content.read_u8()?, version)?;

	let mut endianness: fn([u8; 2]) -> u16 = u16::from_le_bytes;
	if encoding == TextEncoding::UTF16 {
		let mut cursor = Cursor::new(content);
		let mut bom = [0; 2];
		cursor.read_exact(&mut bom)?;

		match [bom[0], bom[1]] {
			[0xFF, 0xFE] => endianness = u16::from_le_bytes,
			[0xFE, 0xFF] => endianness = u16::from_be_bytes,
			// We'll catch an invalid BOM below
			_ => {},
		};

		content = cursor.into_inner();
	}

	let description = decode_text(content, encoding, true)?.unwrap_or_default();

	Ok(Some(if link {
		let content = decode_text(content, TextEncoding::Latin1, false)?.unwrap_or_default();

		FrameValue::UserURL(ExtendedTextFrame {
			encoding,
			description,
			content,
		})
	} else {
		let frame_content;
		// It's possible for the description to be the only string with a BOM
		if encoding == TextEncoding::UTF16 {
			if content.len() >= 2 && (content[..2] == [0xFF, 0xFE] || content[..2] == [0xFE, 0xFF])
			{
				frame_content = decode_text(content, encoding, false)?.unwrap_or_default();
			} else {
				frame_content = match read_to_terminator(content, TextEncoding::UTF16) {
					Some(raw_text) => utf16_decode(&raw_text, endianness).map_err(|_| {
						Into::<LoftyError>::into(ID3v2Error::new(ID3v2ErrorKind::BadSyncText))
					})?,
					None => String::new(),
				}
			}
		} else {
			frame_content = decode_text(content, encoding, false)?.unwrap_or_default();
		}

		FrameValue::UserText(ExtendedTextFrame {
			encoding,
			description,
			content: frame_content,
		})
	}))
}

fn parse_text_language(
	content: &mut &[u8],
	id: &str,
	version: ID3v2Version,
) -> Result<Option<FrameValue>> {
	if content.len() < 5 {
		return Ok(None);
	}

	let encoding = verify_encoding(content.read_u8()?, version)?;

	let mut language = [0; 3];
	content.read_exact(&mut language)?;

	let description = decode_text(content, encoding, true)?;
	let content = decode_text(content, encoding, false)?.unwrap_or_default();

	let information = LanguageFrame {
		encoding,
		language,
		description: description.unwrap_or_default(),
		content,
	};

	let value = match id {
		"COMM" => FrameValue::Comment(information),
		"USLT" => FrameValue::UnSyncText(information),
		_ => unreachable!(),
	};

	Ok(Some(value))
}

fn parse_text(content: &mut &[u8], version: ID3v2Version) -> Result<Option<FrameValue>> {
	if content.len() < 2 {
		return Ok(None);
	}

	let encoding = verify_encoding(content.read_u8()?, version)?;
	let text = decode_text(content, encoding, true)?.unwrap_or_default();

	Ok(Some(FrameValue::Text {
		encoding,
		value: text,
	}))
}

fn parse_link(content: &mut &[u8]) -> Result<Option<FrameValue>> {
	if content.is_empty() {
		return Ok(None);
	}

	let link = decode_text(content, TextEncoding::Latin1, true)?.unwrap_or_default();

	Ok(Some(FrameValue::URL(link)))
}

fn verify_encoding(encoding: u8, version: ID3v2Version) -> Result<TextEncoding> {
	if version == ID3v2Version::V2 && (encoding != 0 && encoding != 1) {
		return Err(ID3v2Error::new(ID3v2ErrorKind::V2InvalidTextEncoding).into());
	}

	match TextEncoding::from_u8(encoding) {
		None => err!(TextDecode("Found invalid encoding")),
		Some(e) => Ok(e),
	}
}
