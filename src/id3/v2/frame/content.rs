use crate::error::{LoftyError, Result};
use crate::id3::v2::frame::FrameValue;
use crate::id3::v2::util::text_utils::{decode_text, encode_text, TextEncoding};
use crate::id3::v2::Id3v2Version;
use crate::types::picture::Picture;

use std::hash::{Hash, Hasher};
use std::io::Read;

use byteorder::ReadBytesExt;

#[derive(Clone, Debug, Eq)]
/// Information about an `ID3v2` frame that requires a language
///
/// See [`EncodedTextFrame`]
pub struct LanguageFrame {
	/// The encoding of the description and comment text
	pub encoding: TextEncoding,
	/// ISO-639-2 language code (3 bytes)
	pub language: String,
	/// Unique content description
	pub description: String,
	/// The actual frame content
	pub content: String,
}

impl PartialEq for LanguageFrame {
	fn eq(&self, other: &Self) -> bool {
		self.description == other.description
	}
}

impl Hash for LanguageFrame {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.description.hash(state);
	}
}

impl LanguageFrame {
	/// Convert a [`LanguageFrame`] to a byte vec
	///
	/// NOTE: This does not include a frame header
	///
	/// # Errors
	///
	/// * `language` is not exactly 3 bytes
	/// * `language` contains invalid characters `('a'..'z')`
	pub fn as_bytes(&self) -> Result<Vec<u8>> {
		let mut bytes = vec![self.encoding as u8];

		if self.language.len() != 3 || self.language.chars().any(|c| !('a'..'z').contains(&c)) {
			return Err(LoftyError::Id3v2(
				"Invalid frame language found (expected 3 ascii characters)",
			));
		}

		bytes.extend(self.language.as_bytes().iter());
		bytes.extend(encode_text(&*self.description, self.encoding, true).iter());
		bytes.extend(encode_text(&*self.content, self.encoding, false));

		Ok(bytes)
	}
}

#[derive(Clone, Debug, Eq)]
/// An `ID3v2` text frame
///
/// This is used in the frames `TXXX` and `WXXX`, where the frames
/// are told apart by descriptions, rather than their [`FrameID`]s.
/// This means for each `EncodedTextFrame` in the tag, the description
/// must be unique.
pub struct EncodedTextFrame {
	/// The encoding of the description and comment text
	pub encoding: TextEncoding,
	/// Unique content description
	pub description: String,
	/// The actual frame content
	pub content: String,
}

impl PartialEq for EncodedTextFrame {
	fn eq(&self, other: &Self) -> bool {
		self.description == other.description
	}
}

impl Hash for EncodedTextFrame {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.description.hash(state);
	}
}

impl EncodedTextFrame {
	/// Convert an [`EncodedTextFrame`] to a byte vec
	pub fn as_bytes(&self) -> Vec<u8> {
		let mut bytes = vec![self.encoding as u8];

		bytes.extend(encode_text(&*self.description, self.encoding, true).iter());
		bytes.extend(encode_text(&*self.content, self.encoding, false));

		bytes
	}
}

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
		"TXXX" => parse_user_defined(content, false)?,
		"WXXX" => parse_user_defined(content, true)?,
		"COMM" | "USLT" => parse_text_language(content, id)?,
		_ if id.starts_with('T') || id == "WFED" => parse_text(content)?,
		// Apple proprietary frames
		"WFED" | "GRP1" => parse_text(content)?,
		_ if id.starts_with('W') => parse_link(content)?,
		// SYLT, GEOB, and any unknown frames
		_ => FrameValue::Binary(content.to_vec()),
	})
}

// There are 2 possibilities for the frame's content: text or link.
fn parse_user_defined(content: &mut &[u8], link: bool) -> Result<FrameValue> {
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

		FrameValue::UserURL(EncodedTextFrame {
			encoding,
			description,
			content,
		})
	} else {
		let content = decode_text(content, encoding, false)?.unwrap_or_else(String::new);

		FrameValue::UserText(EncodedTextFrame {
			encoding,
			description,
			content,
		})
	})
}

fn parse_text_language(content: &mut &[u8], id: &str) -> Result<FrameValue> {
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

	Ok(value)
}

fn parse_text(content: &mut &[u8]) -> Result<FrameValue> {
	let encoding = match TextEncoding::from_u8(content.read_u8()?) {
		None => return Err(LoftyError::TextDecode("Found invalid encoding")),
		Some(e) => e,
	};

	let text = decode_text(content, encoding, true)?.unwrap_or_else(String::new);

	Ok(FrameValue::Text {
		encoding,
		value: text,
	})
}

fn parse_link(content: &mut &[u8]) -> Result<FrameValue> {
	let link = decode_text(content, TextEncoding::Latin1, true)?.unwrap_or_else(String::new);

	Ok(FrameValue::URL(link))
}
