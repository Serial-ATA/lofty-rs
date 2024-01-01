use crate::error::Result;
use crate::id3::v2::frame::content::verify_encoding;
use crate::id3::v2::header::Id3v2Version;
use crate::util::text::{decode_text, encode_text, TextDecodeOptions, TextEncoding};

use byteorder::ReadBytesExt;

use std::io::Read;

/// An `ID3v2` key-value frame
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct KeyValueFrame {
	/// The encoding of the text
	pub encoding: TextEncoding,
	/// The key value pairs. Keys can be specified multiple times
	pub key_value_pairs: Vec<(String, String)>,
}

impl KeyValueFrame {
	/// Read an [`KeyValueFrame`] from a slice
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

		let mut values = Vec::new();

		let mut text_decode_options = TextDecodeOptions::new().encoding(encoding).terminated(true);

		// We have to read the first key/value pair separately because it may be the only string with a BOM

		let first_key = decode_text(reader, text_decode_options)?;

		if first_key.bytes_read == 0 {
			return Ok(None);
		}

		if encoding == TextEncoding::UTF16 {
			text_decode_options = text_decode_options.bom(first_key.bom);
		}

		values.push((
			first_key.content,
			decode_text(reader, text_decode_options)?.content,
		));

		loop {
			let key = decode_text(reader, text_decode_options)?;
			let value = decode_text(reader, text_decode_options)?;
			if key.bytes_read == 0 || value.bytes_read == 0 {
				break;
			}

			values.push((key.content, value.content));
		}

		Ok(Some(Self {
			encoding,
			key_value_pairs: values,
		}))
	}

	/// Convert a [`KeyValueFrame`] to a byte vec
	#[must_use]
	pub fn as_bytes(&self) -> Vec<u8> {
		let mut content = vec![self.encoding as u8];

		for (key, value) in &self.key_value_pairs {
			content.append(&mut encode_text(key, self.encoding, true));
			content.append(&mut encode_text(value, self.encoding, true));
		}
		content
	}
}
