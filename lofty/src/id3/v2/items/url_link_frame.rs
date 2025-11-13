use crate::error::Result;
use crate::id3::v2::{FrameFlags, FrameHeader, FrameId};
use crate::util::text::{TextDecodeOptions, TextEncoding, decode_text, encode_text};

use std::borrow::Cow;
use std::hash::Hash;
use std::io::Read;

/// An `ID3v2` URL frame
#[derive(Clone, Debug, Eq)]
pub struct UrlLinkFrame<'a> {
	pub(crate) header: FrameHeader<'a>,
	pub(crate) content: Cow<'a, str>,
}

impl PartialEq for UrlLinkFrame<'_> {
	fn eq(&self, other: &Self) -> bool {
		self.header.id == other.header.id
	}
}

impl Hash for UrlLinkFrame<'_> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.header.id.hash(state);
	}
}

impl<'a> UrlLinkFrame<'a> {
	/// Create a new [`UrlLinkFrame`]
	pub fn new(id: FrameId<'a>, content: impl Into<Cow<'a, str>>) -> Self {
		UrlLinkFrame {
			header: FrameHeader::new(id, FrameFlags::default()),
			content: content.into(),
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

	/// Read an [`UrlLinkFrame`] from a slice
	///
	/// NOTE: This expects the frame header to have already been skipped
	///
	/// # Errors
	///
	/// * Unable to decode the text as [`TextEncoding::Latin1`]
	pub fn parse<R>(
		reader: &mut R,
		id: FrameId<'a>,
		frame_flags: FrameFlags,
	) -> Result<Option<Self>>
	where
		R: Read,
	{
		let url = decode_text(
			reader,
			TextDecodeOptions::new()
				.encoding(TextEncoding::Latin1)
				.terminated(true),
		)?;
		if url.bytes_read == 0 {
			return Ok(None);
		}

		let header = FrameHeader::new(id, frame_flags);
		Ok(Some(UrlLinkFrame {
			header,
			content: Cow::Owned(url.content),
		}))
	}

	/// Convert an [`UrlLinkFrame`] to a byte vec
	pub fn as_bytes(&self) -> Vec<u8> {
		encode_text(&self.content, TextEncoding::Latin1, false)
	}

	/// Get the URL of the frame
	pub fn url(&self) -> &str {
		&self.content
	}

	/// Change the URL of the frame
	///
	/// This will return a `bool` indicating whether or not the URL provided is Latin-1
	pub fn set_url(&mut self, url: impl Into<Cow<'a, str>>) -> bool {
		let url = url.into();
		if TextEncoding::verify_latin1(&url) {
			self.content = url;
			return true;
		}

		false
	}
}

impl UrlLinkFrame<'static> {
	pub(crate) fn downgrade(&self) -> UrlLinkFrame<'_> {
		UrlLinkFrame {
			header: self.header.downgrade(),
			content: Cow::Borrowed(&self.content),
		}
	}
}
