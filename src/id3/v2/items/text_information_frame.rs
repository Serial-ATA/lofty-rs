use crate::error::Result;
use crate::id3::v2::frame::content::verify_encoding;
use crate::id3::v2::Id3v2Version;
use crate::util::text::{decode_text, encode_text, TextEncoding};

use byteorder::ReadBytesExt;

use std::io::Read;

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
	pub fn parse<R>(reader: &mut R, version: Id3v2Version) -> Result<Option<Self>>
	where
		R: Read,
	{
		let Ok(encoding_byte) = reader.read_u8() else {
			return Ok(None);
		};

		let encoding = verify_encoding(encoding_byte, version)?;
		let value = decode_text(reader, encoding, true)?.content;

		Ok(Some(TextInformationFrame { encoding, value }))
	}

	/// Convert an [`TextInformationFrame`] to a byte vec
	pub fn as_bytes(&self) -> Vec<u8> {
		let mut content = encode_text(&self.value, self.encoding, false);

		content.insert(0, self.encoding as u8);
		content
	}
}
