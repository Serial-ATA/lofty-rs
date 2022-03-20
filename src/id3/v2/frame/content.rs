use crate::error::{ErrorKind, Id3v2Error, Id3v2ErrorKind, LoftyError, Result};
use crate::id3::v2::frame::FrameValue;
use crate::id3::v2::items::encoded_text_frame::EncodedTextFrame;
use crate::id3::v2::items::language_frame::LanguageFrame;
use crate::id3::v2::util::text_utils::{decode_text, TextEncoding};
use crate::id3::v2::Id3v2Version;
use crate::picture::Picture;

use std::io::Read;

use crate::id3::v2::items::popularimeter::Popularimeter;
use byteorder::ReadBytesExt;

pub(super) fn parse_content(
	content: &mut &[u8],
	id: &str,
	version: Id3v2Version,
) -> Result<FrameValue> {
	Ok(match id {
		// The ID was previously upgraded, but the content remains unchanged, so version is necessary
		"APIC" => {
			let (picture, encoding) = Picture::from_apic_bytes(content, version)?;

			FrameValue::Picture { encoding, picture }
		},
		"TXXX" => parse_user_defined(content, false, version)?,
		"WXXX" => parse_user_defined(content, true, version)?,
		"COMM" | "USLT" => parse_text_language(content, id, version)?,
		_ if id.starts_with('T') => parse_text(content, version)?,
		// Apple proprietary frames
		// WFED (Podcast URL), GRP1 (Grouping), MVNM (Movement Name), MVIN (Movement Number)
		"WFED" | "GRP1" | "MVNM" | "MVIN" => parse_text(content, version)?,
		_ if id.starts_with('W') => parse_link(content)?,
		"POPM" => parse_popularimeter(content)?,
		// SYLT, GEOB, and any unknown frames
		_ => FrameValue::Binary(content.to_vec()),
	})
}

// There are 2 possibilities for the frame's content: text or link.
fn parse_user_defined(
	content: &mut &[u8],
	link: bool,
	version: Id3v2Version,
) -> Result<FrameValue> {
	if content.len() < 2 {
		return Err(Id3v2Error::new(Id3v2ErrorKind::BadFrameLength).into());
	}

	let encoding = verify_encoding(content.read_u8()?, version)?;

	let description = decode_text(content, encoding, true)?.unwrap_or_default();

	Ok(if link {
		let content = decode_text(content, TextEncoding::Latin1, false)?.unwrap_or_default();

		FrameValue::UserURL(EncodedTextFrame {
			encoding,
			description,
			content,
		})
	} else {
		let content = decode_text(content, encoding, false)?.unwrap_or_default();

		FrameValue::UserText(EncodedTextFrame {
			encoding,
			description,
			content,
		})
	})
}

fn parse_text_language(content: &mut &[u8], id: &str, version: Id3v2Version) -> Result<FrameValue> {
	if content.len() < 5 {
		return Err(Id3v2Error::new(Id3v2ErrorKind::BadFrameLength).into());
	}

	let encoding = verify_encoding(content.read_u8()?, version)?;

	let mut lang = [0; 3];
	content.read_exact(&mut lang)?;

	let lang = std::str::from_utf8(&lang)
		.map_err(|_| LoftyError::new(ErrorKind::TextDecode("Unable to decode language string")))?;

	let description = decode_text(content, encoding, true)?;
	let content = decode_text(content, encoding, false)?.unwrap_or_default();

	let information = LanguageFrame {
		encoding,
		language: lang.to_string(),
		description: description.unwrap_or_default(),
		content,
	};

	let value = match id {
		"COMM" => FrameValue::Comment(information),
		"USLT" => FrameValue::UnSyncText(information),
		_ => unreachable!(),
	};

	Ok(value)
}

fn parse_text(content: &mut &[u8], version: Id3v2Version) -> Result<FrameValue> {
	let encoding = verify_encoding(content.read_u8()?, version)?;
	let text = decode_text(content, encoding, true)?.unwrap_or_default();

	Ok(FrameValue::Text {
		encoding,
		value: text,
	})
}

fn parse_link(content: &mut &[u8]) -> Result<FrameValue> {
	let link = decode_text(content, TextEncoding::Latin1, true)?.unwrap_or_default();

	Ok(FrameValue::URL(link))
}

fn parse_popularimeter(content: &mut &[u8]) -> Result<FrameValue> {
	let email = decode_text(content, TextEncoding::Latin1, true)?;
	let rating = content.read_u8()?;

	let counter;
	let remaining_size = content.len();
	if remaining_size > 8 {
		counter = u64::MAX;
	} else {
		let mut c = Vec::with_capacity(8);
		content.read_to_end(&mut c)?;

		let needed_zeros = 8 - remaining_size;
		for _ in 0..needed_zeros {
			c.insert(0, 0);
		}

		counter = u64::from_be_bytes(c.try_into().unwrap());
	}

	Ok(FrameValue::Popularimeter(Popularimeter {
		email: email.unwrap_or_default(),
		rating,
		counter,
	}))
}

fn verify_encoding(encoding: u8, version: Id3v2Version) -> Result<TextEncoding> {
	if let Id3v2Version::V2 = version {
		if encoding != 0 && encoding != 1 {
			return Err(Id3v2Error::new(Id3v2ErrorKind::Other(
				"Id3v2.2 only supports Latin-1 and UTF-16 encodings",
			))
			.into());
		}
	}

	match TextEncoding::from_u8(encoding) {
		None => Err(LoftyError::new(ErrorKind::TextDecode(
			"Found invalid encoding",
		))),
		Some(e) => Ok(e),
	}
}
