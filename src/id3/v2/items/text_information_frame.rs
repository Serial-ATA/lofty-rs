use crate::error::Result;
use crate::id3::v2::frame::content::verify_encoding;
use crate::id3::v2::ID3v2Version;
use crate::util::text::{decode_text, encode_text, TextEncoding};

use byteorder::ReadBytesExt;

/// An `ID3v2` text frame
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct TextInformationFrame {
	/// The encoding of the text
	pub encoding: TextEncoding,
	/// The text itself
	pub value: String,
}

impl TextInformationFrame {
	/// Read an [`TextInformationFrame`] from a slice
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
		let text = decode_text(content, encoding, true)?.unwrap_or_default();

		Ok(Some(TextInformationFrame {
			encoding,
			value: text,
		}))
	}

	/// Convert an [`TextInformationFrame`] to a byte vec
	pub fn as_bytes(&self) -> Vec<u8> {
		let mut content = encode_text(&self.value, self.encoding, false);

		content.insert(0, self.encoding as u8);
		content
	}
}
