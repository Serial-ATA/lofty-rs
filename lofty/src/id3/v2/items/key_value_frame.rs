use crate::error::Result;
use crate::id3::v2::frame::content::verify_encoding;
use crate::id3::v2::header::Id3v2Version;
use crate::id3::v2::{FrameFlags, FrameHeader, FrameId};
use crate::util::text::{TextDecodeOptions, TextEncoding, decode_text, encode_text};

use byteorder::ReadBytesExt;

use std::io::Read;

/// An `ID3v2` key-value frame
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct KeyValueFrame<'a> {
	pub(crate) header: FrameHeader<'a>,
	/// The encoding of the text
	pub encoding: TextEncoding,
	/// The key value pairs. Keys can be specified multiple times
	pub key_value_pairs: Vec<(String, String)>,
}

impl<'a> KeyValueFrame<'a> {
	/// Create a new [`KeyValueFrame`]
	pub fn new(
		id: FrameId<'a>,
		encoding: TextEncoding,
		key_value_pairs: Vec<(String, String)>,
	) -> Self {
		let header = FrameHeader::new(id, FrameFlags::default());
		Self {
			header,
			encoding,
			key_value_pairs,
		}
	}

	/// Get the ID for the frame
	pub fn id(&self) -> &FrameId<'_> {
		&self.header.id
	}

	/// Get the flags for the frame
	pub fn flags(&self) -> FrameFlags {
		self.header.flags
	}

	/// Set the flags for the frame
	pub fn set_flags(&mut self, flags: FrameFlags) {
		self.header.flags = flags;
	}

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
	pub fn parse<R>(
		reader: &mut R,
		id: FrameId<'a>,
		frame_flags: FrameFlags,
		version: Id3v2Version,
	) -> Result<Option<Self>>
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

		let header = FrameHeader::new(id, frame_flags);
		Ok(Some(Self {
			header,
			encoding,
			key_value_pairs: values,
		}))
	}

	/// Convert a [`KeyValueFrame`] to a byte vec
	pub fn as_bytes(&self, is_id3v23: bool) -> Vec<u8> {
		let mut encoding = self.encoding;
		if is_id3v23 {
			encoding = encoding.to_id3v23();
		}

		let mut content = vec![encoding as u8];

		for (key, value) in &self.key_value_pairs {
			content.append(&mut encode_text(key, encoding, true));
			content.append(&mut encode_text(value, encoding, true));
		}
		content
	}
}
