use crate::config::WriteOptions;
use crate::error::Result;
use crate::id3::v2::frame::content::verify_encoding;
use crate::id3::v2::header::Id3v2Version;
use crate::id3::v2::{FrameFlags, FrameHeader, FrameId};
use crate::util::text::{TextDecodeOptions, TextEncoding, decode_text};

use std::borrow::Cow;
use std::hash::{Hash, Hasher};
use std::io::Read;

use byteorder::ReadBytesExt;

const FRAME_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("WXXX"));

/// An extended `ID3v2` URL frame
///
/// This is used in the `WXXX` frame, where the frames
/// are told apart by descriptions, rather than their [`FrameId`]s.
/// This means for each `ExtendedUrlFrame` in the tag, the description
/// must be unique.
#[derive(Clone, Debug, Eq)]
pub struct ExtendedUrlFrame<'a> {
	pub(crate) header: FrameHeader<'a>,
	/// The encoding of the description and comment text
	pub encoding: TextEncoding,
	/// Unique content description
	pub description: Cow<'a, str>,
	/// The actual frame content
	pub content: Cow<'a, str>,
}

impl PartialEq for ExtendedUrlFrame<'_> {
	fn eq(&self, other: &Self) -> bool {
		self.description == other.description
	}
}

impl Hash for ExtendedUrlFrame<'_> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.description.hash(state);
	}
}

impl<'a> ExtendedUrlFrame<'a> {
	/// Create a new [`ExtendedUrlFrame`]
	pub fn new(
		encoding: TextEncoding,
		description: impl Into<Cow<'a, str>>,
		content: impl Into<Cow<'a, str>>,
	) -> Self {
		let header = FrameHeader::new(FRAME_ID, FrameFlags::default());
		Self {
			header,
			encoding,
			description: description.into(),
			content: content.into(),
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

	/// Read an [`ExtendedUrlFrame`] from a slice
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
		)?
		.content;
		let content = decode_text(
			reader,
			TextDecodeOptions::new().encoding(TextEncoding::Latin1),
		)?
		.content;

		let header = FrameHeader::new(FRAME_ID, frame_flags);
		Ok(Some(ExtendedUrlFrame {
			header,
			encoding,
			description: Cow::Owned(description),
			content: Cow::Owned(content),
		}))
	}

	/// Convert an [`ExtendedUrlFrame`] to a byte vec
	///
	/// # Errors
	///
	/// * [`WriteOptions::lossy_text_encoding()`] is disabled and the content cannot be encoded in the specified [`TextEncoding`].
	pub fn as_bytes(&self, write_options: WriteOptions) -> Result<Vec<u8>> {
		let mut encoding = self.encoding;
		if write_options.use_id3v23 {
			encoding = encoding.to_id3v23();
		}

		let mut bytes = vec![encoding as u8];

		bytes.extend(
			encoding
				.encode(&self.description, true, write_options.lossy_text_encoding)?
				.iter(),
		);
		bytes.extend(encoding.encode(&self.content, false, write_options.lossy_text_encoding)?);

		Ok(bytes)
	}
}

impl ExtendedUrlFrame<'static> {
	pub(crate) fn downgrade(&self) -> ExtendedUrlFrame<'_> {
		ExtendedUrlFrame {
			header: self.header.downgrade(),
			encoding: self.encoding,
			description: Cow::Borrowed(&self.description),
			content: Cow::Borrowed(&self.content),
		}
	}
}
