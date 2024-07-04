use crate::error::{Id3v2Error, Id3v2ErrorKind, LoftyError, Result};
use crate::id3::v2::frame::content::verify_encoding;
use crate::id3::v2::header::Id3v2Version;
use crate::id3::v2::{FrameFlags, FrameHeader, FrameId};
use crate::macros::err;
use crate::util::text::{
	decode_text, encode_text, utf16_decode_bytes, TextDecodeOptions, TextEncoding,
};

use std::borrow::Cow;
use std::hash::{Hash, Hasher};
use std::io::Read;

use byteorder::ReadBytesExt;

const FRAME_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("TXXX"));

/// An extended `ID3v2` text frame
///
/// This is used in the `TXXX` frame, where the frames
/// are told apart by descriptions, rather than their [`FrameID`](crate::id3::v2::FrameId)s.
/// This means for each `ExtendedTextFrame` in the tag, the description
/// must be unique.
#[derive(Clone, Debug, Eq)]
pub struct ExtendedTextFrame<'a> {
	pub(crate) header: FrameHeader<'a>,
	/// The encoding of the description and comment text
	pub encoding: TextEncoding,
	/// Unique content description
	pub description: String,
	/// The actual frame content
	pub content: String,
}

impl<'a> PartialEq for ExtendedTextFrame<'a> {
	fn eq(&self, other: &Self) -> bool {
		self.description == other.description
	}
}

impl<'a> Hash for ExtendedTextFrame<'a> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.description.hash(state);
	}
}

impl<'a> ExtendedTextFrame<'a> {
	/// Create a new [`ExtendedTextFrame`]
	pub fn new(encoding: TextEncoding, description: String, content: String) -> Self {
		let header = FrameHeader::new(FRAME_ID, FrameFlags::default());
		Self {
			header,
			encoding,
			description,
			content,
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
	pub fn parse<R>(
		reader: &mut R,
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
		let description = decode_text(
			reader,
			TextDecodeOptions::new().encoding(encoding).terminated(true),
		)?;

		let frame_content;
		if encoding != TextEncoding::UTF16 {
			frame_content =
				decode_text(reader, TextDecodeOptions::new().encoding(encoding))?.content;

			let header = FrameHeader::new(FRAME_ID, frame_flags);
			return Ok(Some(ExtendedTextFrame {
				header,
				encoding,
				description: description.content,
				content: frame_content,
			}));
		}

		// It's possible for the description to be the only string with a BOM
		'utf16: {
			let mut raw_text = Vec::new();
			reader.read_to_end(&mut raw_text)?;

			if raw_text.is_empty() {
				// Nothing left to do
				frame_content = String::new();
				break 'utf16;
			}

			// Reuse the BOM from the description as a fallback if the text
			// doesn't specify one.
			let mut bom = description.bom;
			if raw_text.starts_with(&[0xFF, 0xFE]) || raw_text.starts_with(&[0xFE, 0xFF]) {
				// The text specifies a BOM
				bom = [raw_text[0], raw_text[1]];
			}

			let endianness = match bom {
				[0x00, 0x00] if raw_text.is_empty() => {
					debug_assert!(description.content.is_empty());
					// Empty string
					frame_content = String::new();
					break 'utf16;
				},
				[0x00, 0x00] => {
					debug_assert!(description.content.is_empty());
					err!(TextDecode("UTF-16 string has no BOM"));
				},
				[0xFF, 0xFE] => u16::from_le_bytes,
				[0xFE, 0xFF] => u16::from_be_bytes,
				// Handled in description decoding
				_ => unreachable!(),
			};

			frame_content = utf16_decode_bytes(&raw_text, endianness).map_err(|_| {
				Into::<LoftyError>::into(Id3v2Error::new(Id3v2ErrorKind::BadSyncText))
			})?;
		}

		let header = FrameHeader::new(FRAME_ID, frame_flags);
		Ok(Some(ExtendedTextFrame {
			header,
			encoding,
			description: description.content,
			content: frame_content,
		}))
	}

	/// Convert an [`ExtendedTextFrame`] to a byte vec
	pub fn as_bytes(&self, is_id3v23: bool) -> Vec<u8> {
		let mut encoding = self.encoding;
		if is_id3v23 {
			encoding = encoding.to_id3v23();
		}

		let mut bytes = vec![encoding as u8];

		bytes.extend(encode_text(&self.description, encoding, true).iter());
		bytes.extend(encode_text(&self.content, encoding, false));

		bytes
	}
}
