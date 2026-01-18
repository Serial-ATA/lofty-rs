use crate::config::{ParsingMode, WriteOptions};
use crate::error::{Id3v2Error, Id3v2ErrorKind, Result};
use crate::id3::v2::{FrameFlags, FrameHeader, FrameId};
use crate::macros::parse_mode_choice;
use crate::util::text::{TextDecodeOptions, TextEncoding, decode_text};

use std::borrow::Cow;
use std::hash::{Hash, Hasher};
use std::io::Read;

const FRAME_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("UFID"));

/// An `ID3v2` unique file identifier frame (UFID).
#[derive(Clone, Debug, Eq)]
pub struct UniqueFileIdentifierFrame<'a> {
	pub(crate) header: FrameHeader<'a>,
	/// The non-empty owner of the identifier.
	pub owner: Cow<'a, str>,
	/// The binary payload with up to 64 bytes of data.
	pub identifier: Cow<'a, [u8]>,
}

impl PartialEq for UniqueFileIdentifierFrame<'_> {
	fn eq(&self, other: &Self) -> bool {
		self.owner == other.owner
	}
}

impl Hash for UniqueFileIdentifierFrame<'_> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.owner.hash(state);
	}
}

impl<'a> UniqueFileIdentifierFrame<'a> {
	/// Create a new [`UniqueFileIdentifierFrame`]
	pub fn new(owner: impl Into<Cow<'a, str>>, identifier: impl Into<Cow<'a, [u8]>>) -> Self {
		let header = FrameHeader::new(FRAME_ID, FrameFlags::default());
		Self {
			header,
			owner: owner.into(),
			identifier: identifier.into(),
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

	/// Decode the frame contents from bytes
	///
	/// # Errors
	///
	/// Owner is missing or improperly encoded
	pub fn parse<R>(
		reader: &mut R,
		frame_flags: FrameFlags,
		parse_mode: ParsingMode,
	) -> Result<Option<Self>>
	where
		R: Read,
	{
		let owner_decode_result = decode_text(
			reader,
			TextDecodeOptions::new()
				.encoding(TextEncoding::Latin1)
				.terminated(true),
		)?;

		let owner;
		match owner_decode_result.text_or_none() {
			Some(valid) => owner = valid,
			None => {
				parse_mode_choice!(
					parse_mode,
					BESTATTEMPT: owner = String::new(),
					DEFAULT: return Err(Id3v2Error::new(Id3v2ErrorKind::MissingUfidOwner).into())
				);
			},
		}

		let mut identifier = Vec::new();
		reader.read_to_end(&mut identifier)?;

		let header = FrameHeader::new(FRAME_ID, frame_flags);
		Ok(Some(Self {
			header,
			owner: Cow::Owned(owner),
			identifier: Cow::Owned(identifier),
		}))
	}

	/// Encode the frame contents as bytes
	///
	/// # Errors
	///
	/// If [`WriteOptions::lossy_text_encoding()`] is disabled and the `owner` cannot be Latin-1 encoded.
	pub fn as_bytes(&self, write_options: WriteOptions) -> Result<Vec<u8>> {
		let Self {
			owner, identifier, ..
		} = self;

		let mut content = Vec::with_capacity(owner.len() + 1 + identifier.len());
		content.extend(TextEncoding::Latin1.encode(
			owner,
			true,
			write_options.lossy_text_encoding,
		)?);
		content.extend_from_slice(identifier);

		Ok(content)
	}
}

impl UniqueFileIdentifierFrame<'static> {
	pub(crate) fn downgrade(&self) -> UniqueFileIdentifierFrame<'_> {
		UniqueFileIdentifierFrame {
			header: self.header.downgrade(),
			owner: Cow::Borrowed(&self.owner),
			identifier: Cow::Borrowed(&self.identifier),
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::config::WriteOptions;
	use crate::id3::v2::FrameFlags;

	#[test_log::test]
	fn issue_204_invalid_ufid_parsing_mode_best_attempt() {
		use crate::config::ParsingMode;
		use crate::id3::v2::UniqueFileIdentifierFrame;

		let ufid_no_owner = UniqueFileIdentifierFrame::new("", vec![0]);

		let bytes = ufid_no_owner.as_bytes(WriteOptions::default()).unwrap();

		assert!(
			UniqueFileIdentifierFrame::parse(
				&mut &bytes[..],
				FrameFlags::default(),
				ParsingMode::Strict
			)
			.is_err()
		);
		assert!(
			UniqueFileIdentifierFrame::parse(
				&mut &bytes[..],
				FrameFlags::default(),
				ParsingMode::BestAttempt
			)
			.is_ok()
		);
	}
}
