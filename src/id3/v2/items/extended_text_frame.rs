use crate::error::{Id3v2Error, Id3v2ErrorKind, LoftyError, Result};
use crate::id3::v2::frame::content::verify_encoding;
use crate::id3::v2::ID3v2Version;
use crate::util::text::{decode_text, encode_text, read_to_terminator, utf16_decode, TextEncoding};

use std::hash::{Hash, Hasher};
use std::io::{Cursor, Read};

use byteorder::ReadBytesExt;

/// An extended `ID3v2` text frame
///
/// This is used in the `TXXX` frame, where the frames
/// are told apart by descriptions, rather than their [`FrameID`](crate::id3::v2::FrameId)s.
/// This means for each `ExtendedTextFrame` in the tag, the description
/// must be unique.
#[derive(Clone, Debug, Eq)]
pub struct ExtendedTextFrame {
	/// The encoding of the description and comment text
	pub encoding: TextEncoding,
	/// Unique content description
	pub description: String,
	/// The actual frame content
	pub content: String,
}

impl PartialEq for ExtendedTextFrame {
	fn eq(&self, other: &Self) -> bool {
		self.description == other.description
	}
}

impl Hash for ExtendedTextFrame {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.description.hash(state);
	}
}

impl ExtendedTextFrame {
	/// Read an [`ExtendedTextFrame`] from a slice
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

		let mut content = &mut &content[..];
		let encoding = verify_encoding(content.read_u8()?, version)?;

		let mut endianness: fn([u8; 2]) -> u16 = u16::from_le_bytes;
		if encoding == TextEncoding::UTF16 {
			let mut cursor = Cursor::new(content);
			let mut bom = [0; 2];
			cursor.read_exact(&mut bom)?;

			match [bom[0], bom[1]] {
				[0xFF, 0xFE] => endianness = u16::from_le_bytes,
				[0xFE, 0xFF] => endianness = u16::from_be_bytes,
				// We'll catch an invalid BOM below
				_ => {},
			};

			content = cursor.into_inner();
		}

		let description = decode_text(content, encoding, true)?.content;

		let frame_content;
		// It's possible for the description to be the only string with a BOM
		if encoding == TextEncoding::UTF16 {
			if content.len() >= 2 && (content[..2] == [0xFF, 0xFE] || content[..2] == [0xFE, 0xFF])
			{
				frame_content = decode_text(content, encoding, false)?.content;
			} else {
				frame_content = match read_to_terminator(content, TextEncoding::UTF16) {
					Some(raw_text) => utf16_decode(&raw_text, endianness).map_err(|_| {
						Into::<LoftyError>::into(Id3v2Error::new(Id3v2ErrorKind::BadSyncText))
					})?,
					None => String::new(),
				}
			}
		} else {
			frame_content = decode_text(content, encoding, false)?.content;
		}

		Ok(Some(ExtendedTextFrame {
			encoding,
			description,
			content: frame_content,
		}))
	}

	/// Convert an [`ExtendedTextFrame`] to a byte vec
	pub fn as_bytes(&self) -> Vec<u8> {
		let mut bytes = vec![self.encoding as u8];

		bytes.extend(encode_text(&self.description, self.encoding, true).iter());
		bytes.extend(encode_text(&self.content, self.encoding, false));

		bytes
	}
}
