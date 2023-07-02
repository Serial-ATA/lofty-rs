use crate::error::Result;
use crate::id3::v2::frame::content::verify_encoding;
use crate::id3::v2::Id3v2Version;
use crate::util::text::{decode_text, encode_text, TextEncoding};

use byteorder::ReadBytesExt;

use std::io::Read;

/// An `ID3v2` text frame
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct KeyValueFrame {
	/// The encoding of the text
	pub encoding: TextEncoding,
	/// The text itself
	pub values: Vec<(String, String)>,
}

impl KeyValueFrame {
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
        
        let mut values = vec![];

        loop {
            let key = decode_text(reader, encoding, true)?;
            let value = decode_text(reader, encoding, true)?;
            if key.bytes_read == 0 || value.bytes_read == 0 {
                break;
            }

            values.push((key.content, value.content));
        }


		Ok(Some(Self{ encoding, values }))
	}

	/// Convert an [`TextInformationFrame`] to a byte vec
	pub fn as_bytes(&self) -> Vec<u8> {
		let mut content = vec![];
        
        for (key, value) in &self.values {
            content.append(&mut encode_text(key, self.encoding, true));
            content.append(&mut encode_text(value, self.encoding, true));
        }

		content.insert(0, self.encoding as u8);
		content
	}
}
