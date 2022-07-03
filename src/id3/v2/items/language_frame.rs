use crate::error::{ID3v2Error, ID3v2ErrorKind, Result};
use crate::id3::v2::util::text_utils::{encode_text, TextEncoding};

use std::hash::{Hash, Hasher};

/// Information about an `ID3v2` frame that requires a language
///
/// See [`EncodedTextFrame`](crate::id3::v2::EncodedTextFrame)
#[derive(Clone, Debug, Eq)]
pub struct LanguageFrame {
	/// The encoding of the description and comment text
	pub encoding: TextEncoding,
	// TODO: Make this [u8; 3] in the next major release, since it isn't always a valid UTF-8 string
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

		if self.language.len() != 3 || self.language.chars().any(|c| !('a'..='z').contains(&c)) {
			return Err(ID3v2Error::new(ID3v2ErrorKind::Other(
				"Invalid frame language found (expected 3 ascii characters)",
			))
			.into());
		}

		bytes.extend(self.language.as_bytes().iter());
		bytes.extend(encode_text(&self.description, self.encoding, true).iter());
		bytes.extend(encode_text(&self.content, self.encoding, false));

		Ok(bytes)
	}
}
