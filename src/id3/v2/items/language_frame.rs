use crate::error::{Id3v2Error, Id3v2ErrorKind, Result};
use crate::id3::v2::frame::content::verify_encoding;
use crate::id3::v2::ID3v2Version;
use crate::util::text::{decode_text, encode_text, TextEncoding};

use std::hash::{Hash, Hasher};
use std::io::Read;

use byteorder::ReadBytesExt;

// Generic struct for a text frame that has a language
//
// This exists to deduplicate some code between `CommentFrame` and `UnsynchronizedTextFrame`
struct LanguageFrame {
	pub encoding: TextEncoding,
	pub language: [u8; 3],
	pub description: String,
	pub content: String,
}

impl LanguageFrame {
	fn parse(content: &[u8], version: ID3v2Version) -> Result<Option<Self>> {
		if content.len() < 5 {
			return Ok(None);
		}

		let content = &mut &content[..];
		let encoding = verify_encoding(content.read_u8()?, version)?;

		let mut language = [0; 3];
		content.read_exact(&mut language)?;

		let description = decode_text(content, encoding, true)?.unwrap_or_default();
		let content = decode_text(content, encoding, false)?.unwrap_or_default();

		Ok(Some(Self {
			encoding,
			language,
			description,
			content,
		}))
	}

	fn create_bytes(
		encoding: TextEncoding,
		language: [u8; 3],
		description: &str,
		content: &str,
	) -> Result<Vec<u8>> {
		let mut bytes = vec![encoding as u8];

		if language.len() != 3 || language.iter().any(|c| !c.is_ascii_alphabetic()) {
			return Err(Id3v2Error::new(Id3v2ErrorKind::InvalidLanguage(language)).into());
		}

		bytes.extend(language);
		bytes.extend(encode_text(description, encoding, true).iter());
		bytes.extend(encode_text(content, encoding, false));

		Ok(bytes)
	}
}

/// An `ID3v2` comment frame
///
/// Similar to `TXXX` and `WXXX` frames, comments are told apart by their descriptions.
#[derive(Clone, Debug, Eq)]
pub struct CommentFrame {
	/// The encoding of the description and comment text
	pub encoding: TextEncoding,
	/// ISO-639-2 language code (3 bytes)
	pub language: [u8; 3],
	/// Unique content description
	pub description: String,
	/// The actual frame content
	pub content: String,
}

impl PartialEq for CommentFrame {
	fn eq(&self, other: &Self) -> bool {
		self.description == other.description
	}
}

impl Hash for CommentFrame {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.description.hash(state);
	}
}

impl From<LanguageFrame> for CommentFrame {
	fn from(value: LanguageFrame) -> Self {
		Self {
			encoding: value.encoding,
			language: value.language,
			description: value.description,
			content: value.content,
		}
	}
}

impl CommentFrame {
	/// Read a [`CommentFrame`] from a slice
	///
	/// NOTE: This expects the frame header to have already been skipped
	///
	/// # Errors
	///
	/// * Unable to decode the text
	///
	/// ID3v2.2:
	///
	/// * The encoding is not [`TextEncoding::Latin1`] or [`TextEncoding::UTF16`]
	pub fn parse(content: &[u8], version: ID3v2Version) -> Result<Option<Self>> {
		Ok(LanguageFrame::parse(content, version)?.map(Into::into))
	}

	/// Convert a [`CommentFrame`] to a byte vec
	///
	/// NOTE: This does not include a frame header
	///
	/// # Errors
	///
	/// * `language` is not exactly 3 bytes
	/// * `language` contains invalid characters (Only `'a'..='z'` and `'A'..='Z'` allowed)
	pub fn as_bytes(&self) -> Result<Vec<u8>> {
		LanguageFrame::create_bytes(
			self.encoding,
			self.language,
			&self.description,
			&self.content,
		)
	}
}

/// An `ID3v2` unsynchronized lyrics/text frame
///
/// Similar to `TXXX` and `WXXX` frames, USLT frames are told apart by their descriptions.
#[derive(Clone, Debug, Eq)]
pub struct UnsynchronizedTextFrame {
	/// The encoding of the description and content
	pub encoding: TextEncoding,
	/// ISO-639-2 language code (3 bytes)
	pub language: [u8; 3],
	/// Unique content description
	pub description: String,
	/// The actual frame content
	pub content: String,
}

impl PartialEq for UnsynchronizedTextFrame {
	fn eq(&self, other: &Self) -> bool {
		self.description == other.description
	}
}

impl Hash for UnsynchronizedTextFrame {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.description.hash(state);
	}
}

impl From<LanguageFrame> for UnsynchronizedTextFrame {
	fn from(value: LanguageFrame) -> Self {
		Self {
			encoding: value.encoding,
			language: value.language,
			description: value.description,
			content: value.content,
		}
	}
}

impl UnsynchronizedTextFrame {
	/// Read a [`UnsynchronizedTextFrame`] from a slice
	///
	/// NOTE: This expects the frame header to have already been skipped
	///
	/// # Errors
	///
	/// * Unable to decode the text
	///
	/// ID3v2.2:
	///
	/// * The encoding is not [`TextEncoding::Latin1`] or [`TextEncoding::UTF16`]
	pub fn parse(content: &[u8], version: ID3v2Version) -> Result<Option<Self>> {
		Ok(LanguageFrame::parse(content, version)?.map(Into::into))
	}

	/// Convert a [`UnsynchronizedTextFrame`] to a byte vec
	///
	/// NOTE: This does not include a frame header
	///
	/// # Errors
	///
	/// * `language` is not exactly 3 bytes
	/// * `language` contains invalid characters (Only `'a'..='z'` and `'A'..='Z'` allowed)
	pub fn as_bytes(&self) -> Result<Vec<u8>> {
		LanguageFrame::create_bytes(
			self.encoding,
			self.language,
			&self.description,
			&self.content,
		)
	}
}
