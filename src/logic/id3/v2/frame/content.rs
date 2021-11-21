use crate::error::{LoftyError, Result};
use crate::logic::id3::v2::frame::{EncodedTextFrame, FrameID, FrameValue, LanguageFrame};
use crate::logic::id3::v2::util::text_utils::{decode_text, TextEncoding};
use crate::logic::id3::v2::Id3v2Version;
use crate::types::picture::Picture;

use std::io::Read;

use byteorder::ReadBytesExt;

pub(crate) fn parse_content(
	content: &mut &[u8],
	id: &str,
	version: Id3v2Version,
) -> Result<(FrameID, FrameValue)> {
	Ok(match id {
		// The ID was previously upgraded, but the content remains unchanged, so version is necessary
		"APIC" => (
			FrameID::Valid(String::from("APIC")),
			FrameValue::Picture(Picture::from_apic_bytes(content, version)?),
		),
		"TXXX" => parse_user_defined(content, false)?,
		"WXXX" => parse_user_defined(content, true)?,
		"COMM" | "USLT" => parse_text_language(id, content)?,
		_ if id.starts_with('T') => parse_text(id, content)?,
		_ if id.starts_with('W') => parse_link(id, content)?,
		// SYLT, GEOB, and any unknown frames
		_ => (
			FrameID::Valid(String::from(id)),
			FrameValue::Binary(content.to_vec()),
		),
	})
}

// There are 2 possibilities for the frame's content: text or link.
fn parse_user_defined(content: &mut &[u8], link: bool) -> Result<(FrameID, FrameValue)> {
	if content.len() < 2 {
		return Err(LoftyError::BadFrameLength);
	}

	let encoding = match TextEncoding::from_u8(content.read_u8()?) {
		None => return Err(LoftyError::TextDecode("Found invalid encoding")),
		Some(e) => e,
	};

	let description = decode_text(content, encoding, true)?.unwrap_or_else(String::new);

	Ok(if link {
		let content =
			decode_text(content, TextEncoding::Latin1, false)?.unwrap_or_else(String::new);

		(
			FrameID::Valid(String::from("WXXX")),
			FrameValue::UserURL(EncodedTextFrame {
				encoding,
				description,
				content,
			}),
		)
	} else {
		let content = decode_text(content, encoding, false)?.unwrap_or_else(String::new);

		(
			FrameID::Valid(String::from("TXXX")),
			FrameValue::UserText(EncodedTextFrame {
				encoding,
				description,
				content,
			}),
		)
	})
}

fn parse_text_language(id: &str, content: &mut &[u8]) -> Result<(FrameID, FrameValue)> {
	if content.len() < 5 {
		return Err(LoftyError::BadFrameLength);
	}

	let encoding = match TextEncoding::from_u8(content.read_u8()?) {
		None => return Err(LoftyError::TextDecode("Found invalid encoding")),
		Some(e) => e,
	};

	let mut lang = [0; 3];
	content.read_exact(&mut lang)?;

	let lang = std::str::from_utf8(&lang)
		.map_err(|_| LoftyError::TextDecode("Unable to decode language string"))?;

	let description = decode_text(content, encoding, true)?;
	let content = decode_text(content, encoding, false)?.unwrap_or_else(String::new);

	let information = LanguageFrame {
		encoding,
		language: lang.to_string(),
		description: description.unwrap_or_else(|| String::from("")),
		content,
	};

	let value = match id {
		"COMM" => FrameValue::Comment(information),
		"USLT" => FrameValue::UnSyncText(information),
		_ => unreachable!(),
	};

	let id = FrameID::Valid(String::from(id));

	Ok((id, value))
}

fn parse_text(id: &str, content: &mut &[u8]) -> Result<(FrameID, FrameValue)> {
	let encoding = match TextEncoding::from_u8(content.read_u8()?) {
		None => return Err(LoftyError::TextDecode("Found invalid encoding")),
		Some(e) => e,
	};

	let text = decode_text(content, encoding, false)?.unwrap_or_else(String::new);

	Ok((
		FrameID::Valid(String::from(id)),
		FrameValue::Text {
			encoding,
			value: text,
		},
	))
}

fn parse_link(id: &str, content: &mut &[u8]) -> Result<(FrameID, FrameValue)> {
	let link = decode_text(content, TextEncoding::Latin1, false)?.unwrap_or_else(String::new);

	Ok((FrameID::Valid(String::from(id)), FrameValue::URL(link)))
}
