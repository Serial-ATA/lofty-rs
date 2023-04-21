use crate::error::Result;
use crate::id3::v2::frame::content::verify_encoding;
use crate::id3::v2::ID3v2Version;
use crate::util::text::{decode_text, encode_text, TextEncoding};

use std::hash::{Hash, Hasher};

use byteorder::ReadBytesExt;

/// An extended `ID3v2` URL frame
///
/// This is used in the `WXXX` frame, where the frames
/// are told apart by descriptions, rather than their [`FrameId`](crate::id3::v2::FrameId)s.
/// This means for each `ExtendedUrlFrame` in the tag, the description
/// must be unique.
#[derive(Clone, Debug, Eq)]
pub struct ExtendedUrlFrame {
	/// The encoding of the description and comment text
	pub encoding: TextEncoding,
	/// Unique content description
	pub description: String,
	/// The actual frame content
	pub content: String,
}

impl PartialEq for ExtendedUrlFrame {
	fn eq(&self, other: &Self) -> bool {
		self.description == other.description
	}
}

impl Hash for ExtendedUrlFrame {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.description.hash(state);
	}
}

impl ExtendedUrlFrame {
	/// Read an [`ExtendedUrlFrame`] from a slice
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
		if content.len() < 2 {
			return Ok(None);
		}

		let content = &mut &content[..];

		let encoding = verify_encoding(content.read_u8()?, version)?;
		let description = decode_text(content, encoding, true)?.unwrap_or_default();
		let content = decode_text(content, TextEncoding::Latin1, false)?.unwrap_or_default();

		Ok(Some(ExtendedUrlFrame {
			encoding,
			description,
			content,
		}))
	}

	/// Convert an [`ExtendedUrlFrame`] to a byte vec
	pub fn as_bytes(&self) -> Vec<u8> {
		let mut bytes = vec![self.encoding as u8];

		bytes.extend(encode_text(&self.description, self.encoding, true).iter());
		bytes.extend(encode_text(&self.content, self.encoding, false));

		bytes
	}
}
