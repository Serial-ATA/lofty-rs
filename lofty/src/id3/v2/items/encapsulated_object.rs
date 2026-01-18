use crate::config::WriteOptions;
use crate::error::{ErrorKind, Id3v2Error, Id3v2ErrorKind, LoftyError, Result};
use crate::id3::v2::{FrameFlags, FrameHeader, FrameId};
use crate::util::text::{TextDecodeOptions, TextEncoding, decode_text};

use std::io::{Cursor, Read};

const FRAME_ID: FrameId<'static> = FrameId::Valid(std::borrow::Cow::Borrowed("GEOB"));

/// Allows for encapsulation of any file type inside an ID3v2 tag
#[derive(PartialEq, Clone, Debug, Eq, Hash)]
pub struct GeneralEncapsulatedObject<'a> {
	pub(crate) header: FrameHeader<'a>,
	/// The text encoding of `file_name` and `description`
	pub encoding: TextEncoding,
	/// The file's mimetype
	pub mime_type: Option<String>,
	/// The file's name
	pub file_name: Option<String>,
	/// A unique content descriptor
	pub descriptor: Option<String>,
	/// The file's content
	pub data: Vec<u8>,
}

impl GeneralEncapsulatedObject<'_> {
	/// Create a new [`GeneralEncapsulatedObject`]
	pub fn new(
		encoding: TextEncoding,
		mime_type: Option<String>,
		file_name: Option<String>,
		descriptor: Option<String>,
		data: Vec<u8>,
	) -> Self {
		let header = FrameHeader::new(FRAME_ID, FrameFlags::default());
		Self {
			header,
			encoding,
			mime_type,
			file_name,
			descriptor,
			data,
		}
	}

	/// Get the ID for the frame
	pub fn id(&self) -> FrameId<'_> {
		FRAME_ID
	}

	/// Get the flags for the frame
	pub fn flags(&self) -> FrameFlags {
		self.header.flags
	}

	/// Set the flags for the frame
	pub fn set_flags(&mut self, flags: FrameFlags) {
		self.header.flags = flags;
	}

	/// Read a [`GeneralEncapsulatedObject`] from a slice
	///
	/// NOTE: This expects the frame header to have already been skipped
	///
	/// # Errors
	///
	/// This function will return an error if at any point it's unable to parse the data
	pub fn parse(data: &[u8], frame_flags: FrameFlags) -> Result<Self> {
		if data.len() < 4 {
			return Err(Id3v2Error::new(Id3v2ErrorKind::BadFrameLength).into());
		}

		let encoding = TextEncoding::from_u8(data[0])
			.ok_or_else(|| LoftyError::new(ErrorKind::TextDecode("Found invalid encoding")))?;

		let mut cursor = Cursor::new(&data[1..]);

		let mime_type = decode_text(
			&mut cursor,
			TextDecodeOptions::new()
				.encoding(TextEncoding::Latin1)
				.terminated(true),
		)?;

		let text_decode_options = TextDecodeOptions::new().encoding(encoding).terminated(true);

		let file_name = decode_text(&mut cursor, text_decode_options)?;
		let descriptor = decode_text(&mut cursor, text_decode_options)?;

		let mut data = Vec::new();
		cursor.read_to_end(&mut data)?;

		let header = FrameHeader::new(FRAME_ID, frame_flags);
		Ok(Self {
			header,
			encoding,
			mime_type: mime_type.text_or_none(),
			file_name: file_name.text_or_none(),
			descriptor: descriptor.text_or_none(),
			data,
		})
	}

	/// Convert a [`GeneralEncapsulatedObject`] into an ID3v2 GEOB frame byte Vec
	///
	/// NOTE: This does not include a frame header
	///
	/// # Errors
	///
	/// * [`WriteOptions::lossy_text_encoding()`] is disabled and the content cannot be encoded in the specified [`TextEncoding`].
	pub fn as_bytes(&self, write_options: WriteOptions) -> Result<Vec<u8>> {
		let encoding = self.encoding;

		let mut bytes = vec![encoding as u8];

		if let Some(ref mime_type) = self.mime_type {
			bytes.extend(mime_type.as_bytes())
		}

		bytes.push(0);

		let file_name = self.file_name.as_deref();
		bytes.extend(&*encoding.encode(
			file_name.unwrap_or(""),
			true,
			write_options.lossy_text_encoding,
		)?);

		let descriptor = self.descriptor.as_deref();
		bytes.extend(&*encoding.encode(
			descriptor.unwrap_or(""),
			true,
			write_options.lossy_text_encoding,
		)?);

		bytes.extend(&self.data);

		Ok(bytes)
	}
}

#[cfg(test)]
mod tests {
	use crate::config::WriteOptions;
	use crate::id3::v2::{FrameFlags, FrameHeader, GeneralEncapsulatedObject};
	use crate::util::text::TextEncoding;

	fn expected() -> GeneralEncapsulatedObject<'static> {
		GeneralEncapsulatedObject {
			header: FrameHeader::new(super::FRAME_ID, FrameFlags::default()),
			encoding: TextEncoding::Latin1,
			mime_type: Some(String::from("audio/mpeg")),
			file_name: Some(String::from("a.mp3")),
			descriptor: Some(String::from("Test Asset")),
			data: crate::tag::utils::test_utils::read_path(
				"tests/files/assets/minimal/full_test.mp3",
			),
		}
	}

	#[test_log::test]
	fn geob_decode() {
		let expected = expected();

		let cont = crate::tag::utils::test_utils::read_path("tests/tags/assets/id3v2/test.geob");

		let parsed_geob = GeneralEncapsulatedObject::parse(&cont, FrameFlags::default()).unwrap();

		assert_eq!(parsed_geob, expected);
	}

	#[test_log::test]
	fn geob_encode() {
		let to_encode = expected();

		let encoded = to_encode.as_bytes(WriteOptions::default()).unwrap();

		let expected_bytes =
			crate::tag::utils::test_utils::read_path("tests/tags/assets/id3v2/test.geob");

		assert_eq!(encoded, expected_bytes);
	}
}
