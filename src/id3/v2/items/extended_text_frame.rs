use crate::error::{Id3v2Error, Id3v2ErrorKind, LoftyError, Result};
use crate::id3::v2::frame::content::verify_encoding;
use crate::id3::v2::Id3v2Version;
use crate::util::text::{decode_text, encode_text, read_to_terminator, utf16_decode, TextEncoding};

use std::hash::{Hash, Hasher};
use std::io::Read;

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
	pub fn parse<R>(reader: &mut R, version: Id3v2Version) -> Result<Option<Self>>
	where
		R: Read,
	{
		let Ok(encoding_byte) = reader.read_u8() else {
			return Ok(None);
		};

		let encoding = verify_encoding(encoding_byte, version)?;
		let description = decode_text(reader, encoding, true)?;

		let frame_content;
		if encoding != TextEncoding::UTF16 {
			frame_content = decode_text(reader, encoding, false)?.content;

			return Ok(Some(ExtendedTextFrame {
				encoding,
				description: description.content,
				content: frame_content,
			}));
		}

		// It's possible for the description to be the only string with a BOM
		'utf16: {
			let bom = description.bom;
			let Some(raw_text) = read_to_terminator(reader, TextEncoding::UTF16) else {
				// Nothing left to do
				frame_content = String::new();
				break 'utf16;
			};

			if raw_text.starts_with(&[0xFF, 0xFE]) || raw_text.starts_with(&[0xFE, 0xFF]) {
				frame_content =
					decode_text(&mut &raw_text[..], TextEncoding::UTF16, false)?.content;
				break 'utf16;
			}

			let endianness = match bom {
				[0xFF, 0xFE] => u16::from_le_bytes,
				[0xFE, 0xFF] => u16::from_be_bytes,
				// Handled in description decoding
				_ => unreachable!(),
			};

			frame_content = utf16_decode(&raw_text, endianness).map_err(|_| {
				Into::<LoftyError>::into(Id3v2Error::new(Id3v2ErrorKind::BadSyncText))
			})?;
		}

		Ok(Some(ExtendedTextFrame {
			encoding,
			description: description.content,
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
