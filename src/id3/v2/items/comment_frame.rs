use crate::error::{ID3v2Error, ID3v2ErrorKind, Result};
use crate::util::text::{encode_text, TextEncoding};

use std::hash::{Hash, Hasher};

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

impl CommentFrame {
	/// Convert a [`CommentFrame`] to a byte vec
	///
	/// NOTE: This does not include a frame header
	///
	/// # Errors
	///
	/// * `language` is not exactly 3 bytes
	/// * `language` contains invalid characters (Only `'a'..='z'` and `'A'..='Z'` allowed)
	pub fn as_bytes(&self) -> Result<Vec<u8>> {
		let mut bytes = vec![self.encoding as u8];

		if self.language.len() != 3 || self.language.iter().any(|c| !c.is_ascii_alphabetic()) {
			return Err(ID3v2Error::new(ID3v2ErrorKind::InvalidLanguage(self.language)).into());
		}

		bytes.extend(self.language);
		bytes.extend(encode_text(&self.description, self.encoding, true).iter());
		bytes.extend(encode_text(&self.content, self.encoding, false));

		Ok(bytes)
	}
}
