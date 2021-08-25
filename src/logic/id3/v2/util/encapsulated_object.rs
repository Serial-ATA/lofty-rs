use crate::error::{LoftyError, Result};
use crate::logic::id3::v2::util::text_utils::decode_text;
use crate::types::picture::TextEncoding;

use std::io::{Cursor, Read};

#[derive(PartialEq, Clone)]
/// Information about a [`GeneralEncapsulatedObject`]
pub struct GEOBInformation {
	/// The text encoding of `file_name` and `description`
	pub encoding: TextEncoding,
	/// The file's mimetype
	pub mime_type: Option<String>,
	/// The file's name
	pub file_name: Option<String>,
	/// A description of the content
	pub description: String,
}

/// Allows for encapsulation of any file type inside an ID3v2 tag
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
		let description = decode_text(&mut cursor, encoding, true)?.unwrap_or_else(String::new);

		let mut data = Vec::new();
		cursor.read_to_end(&mut data)?;

		Ok(Self {
			information: GEOBInformation {
				encoding,
				mime_type,
				file_name,
				description,
			},
			data,
		})
	}
}
