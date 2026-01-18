use crate::config::WriteOptions;
use crate::error::Result;
use crate::id3::v2::frame::content::verify_encoding;
use crate::id3::v2::header::Id3v2Version;
use crate::id3::v2::{FrameFlags, FrameHeader, FrameId};
use crate::util::text::{TextDecodeOptions, TextEncoding, decode_text};

use byteorder::ReadBytesExt;

use std::borrow::Cow;
use std::hash::Hash;
use std::io::Read;

/// An `ID3v2` text frame
#[derive(Clone, Debug, Eq)]
pub struct TextInformationFrame<'a> {
	pub(crate) header: FrameHeader<'a>,
	/// The encoding of the text
	pub encoding: TextEncoding,
	/// The text itself
	pub value: Cow<'a, str>,
}

impl PartialEq for TextInformationFrame<'_> {
	fn eq(&self, other: &Self) -> bool {
		self.header.id == other.header.id
	}
}

impl Hash for TextInformationFrame<'_> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.header.id.hash(state);
	}
}

impl<'a> TextInformationFrame<'a> {
	/// Create a new [`TextInformationFrame`]
	pub fn new(id: FrameId<'a>, encoding: TextEncoding, value: impl Into<Cow<'a, str>>) -> Self {
		let header = FrameHeader::new(id, FrameFlags::default());
		Self {
			header,
			encoding,
			value: value.into(),
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
		let value = decode_text(reader, TextDecodeOptions::new().encoding(encoding))?.content;

		let header = FrameHeader::new(id, frame_flags);
		Ok(Some(TextInformationFrame {
			header,
			encoding,
			value: Cow::Owned(value),
		}))
	}

	/// Convert an [`TextInformationFrame`] to a byte vec
	///
	/// # Errors
	///
	/// * [`WriteOptions::lossy_text_encoding()`] is disabled and the content cannot be encoded in the specified [`TextEncoding`].
	pub fn as_bytes(&self, write_options: WriteOptions) -> Result<Vec<u8>> {
		let mut encoding = self.encoding;
		if write_options.use_id3v23 {
			encoding = encoding.to_id3v23();
		}

		let mut content = encoding.encode(&self.value, false, write_options.lossy_text_encoding)?;
		content.insert(0, encoding as u8);
		Ok(content)
	}
}

impl TextInformationFrame<'static> {
	pub(crate) fn downgrade(&self) -> TextInformationFrame<'_> {
		TextInformationFrame {
			header: self.header.downgrade(),
			encoding: self.encoding,
			value: Cow::Borrowed(&self.value),
		}
	}
}
