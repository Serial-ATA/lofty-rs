use crate::error::{LoftyError, Result};
use crate::id3::v2::util::text_utils::{decode_text, encode_text, TextEncoding};

use std::io::{Cursor, Read};

#[derive(PartialEq, Clone, Debug, Eq, Hash)]
/// Information about a [`GeneralEncapsulatedObject`]
pub struct GEOBInformation {
	/// The text encoding of `file_name` and `description`
	pub encoding: TextEncoding,
	/// The file's mimetype
	pub mime_type: Option<String>,
	/// The file's name
	pub file_name: Option<String>,
	/// A unique content descriptor
	pub descriptor: Option<String>,
}

/// Allows for encapsulation of any file type inside an ID3v2 tag
#[derive(PartialEq, Clone, Debug, Eq, Hash)]
pub struct GeneralEncapsulatedObject {
	/// Information about the data
	pub information: GEOBInformation,
	/// The file's content
	pub data: Vec<u8>,
}

impl GeneralEncapsulatedObject {
	/// Read a [`GeneralEncapsulatedObject`] from a slice
	///
	/// NOTE: This expects the frame header to have already been skipped
	///
	/// # Errors
	///
	/// This function will return an error if at any point it's unable to parse the data
	pub fn parse(data: &[u8]) -> Result<Self> {
		if data.len() < 4 {
			return Err(LoftyError::Id3v2("GEOB frame has invalid size (< 4)"));
		}

		let encoding = TextEncoding::from_u8(data[0])
			.ok_or(LoftyError::TextDecode("Found invalid encoding"))?;

		let mut cursor = Cursor::new(&data[1..]);

		let mime_type = decode_text(&mut cursor, TextEncoding::Latin1, true)?;
		let file_name = decode_text(&mut cursor, encoding, true)?;
		let descriptor = decode_text(&mut cursor, encoding, true)?;

		let mut data = Vec::new();
		cursor.read_to_end(&mut data)?;

		Ok(Self {
			information: GEOBInformation {
				encoding,
				mime_type,
				file_name,
				descriptor,
			},
			data,
		})
	}

	/// Convert a [`GeneralEncapsulatedObject`] into an ID3v2 GEOB frame byte Vec
	///
	/// NOTE: This does not include a frame header
	pub fn as_bytes(&self) -> Vec<u8> {
		let encoding = self.information.encoding;

		let mut bytes = vec![encoding as u8];

		if let Some(ref mime_type) = self.information.mime_type {
			bytes.extend(mime_type.as_bytes())
		}

		bytes.push(0);

		let file_name = self.information.file_name.as_deref();
		bytes.extend(&*encode_text(file_name.unwrap_or(""), encoding, true));

		let descriptor = self.information.descriptor.as_deref();
		bytes.extend(&*encode_text(descriptor.unwrap_or(""), encoding, true));

		bytes.extend(&self.data);

		bytes
	}
}

#[cfg(test)]
mod tests {
	use crate::id3::v2::{GEOBInformation, GeneralEncapsulatedObject, TextEncoding};

	#[test]
	fn geob_decode() {
		let expected = GeneralEncapsulatedObject {
			information: GEOBInformation {
				encoding: TextEncoding::Latin1,
				mime_type: Some(String::from("audio/mpeg")),
				file_name: Some(String::from("a.mp3")),
				descriptor: Some(String::from("Test Asset")),
			},
			data: crate::tag_utils::test_utils::read_path("tests/files/assets/a.mp3"),
		};

		let cont = crate::tag_utils::test_utils::read_path("tests/tags/assets/id3v2/test.geob");

		let parsed_geob = GeneralEncapsulatedObject::parse(&*cont).unwrap();

		assert_eq!(parsed_geob, expected);
	}

	#[test]
	fn geob_encode() {
		let to_encode = GeneralEncapsulatedObject {
			information: GEOBInformation {
				encoding: TextEncoding::Latin1,
				mime_type: Some(String::from("audio/mpeg")),
				file_name: Some(String::from("a.mp3")),
				descriptor: Some(String::from("Test Asset")),
			},
			data: crate::tag_utils::test_utils::read_path("tests/files/assets/a.mp3"),
		};

		let encoded = to_encode.as_bytes();

		let expected_bytes =
			crate::tag_utils::test_utils::read_path("tests/tags/assets/id3v2/test.geob");

		assert_eq!(encoded, expected_bytes);
	}
}
