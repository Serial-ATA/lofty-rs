use crate::config::ParsingMode;
use crate::error::{Id3v2Error, Id3v2ErrorKind, Result};
use crate::id3::v2::{FrameFlags, FrameHeader, FrameId};
use crate::macros::parse_mode_choice;
use crate::util::text::{TextDecodeOptions, TextEncoding, decode_text, encode_text};

use std::borrow::Cow;
use std::hash::{Hash, Hasher};
use std::io::Read;

const FRAME_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("UFID"));

/// An `ID3v2` unique file identifier frame (UFID).
#[derive(Clone, Debug, Eq)]
pub struct UniqueFileIdentifierFrame<'a> {
	pub(crate) header: FrameHeader<'a>,
	/// The non-empty owner of the identifier.
	pub owner: String,
	/// The binary payload with up to 64 bytes of data.
	pub identifier: Vec<u8>,
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

impl UniqueFileIdentifierFrame<'_> {
	/// Create a new [`UniqueFileIdentifierFrame`]
	pub fn new(owner: String, identifier: Vec<u8>) -> Self {
		let header = FrameHeader::new(FRAME_ID, FrameFlags::default());
		Self {
			header,
			owner,
			identifier,
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
			owner,
			identifier,
		}))
	}

	/// Encode the frame contents as bytes
	pub fn as_bytes(&self) -> Vec<u8> {
		let Self {
			owner, identifier, ..
		} = self;

		let mut content = Vec::with_capacity(owner.len() + 1 + identifier.len());
		content.extend(encode_text(owner.as_str(), TextEncoding::Latin1, true));
		content.extend_from_slice(identifier);

		content
	}
}

#[cfg(test)]
mod tests {
	use crate::id3::v2::{FrameFlags, FrameHeader, FrameId};

	use std::borrow::Cow;

	#[test_log::test]
	fn issue_204_invalid_ufid_parsing_mode_best_attempt() {
		use crate::config::ParsingMode;
		use crate::id3::v2::UniqueFileIdentifierFrame;

		let ufid_no_owner = UniqueFileIdentifierFrame {
			header: FrameHeader::new(FrameId::Valid(Cow::Borrowed("UFID")), FrameFlags::default()),
			owner: String::new(),
			identifier: vec![0],
		};

		let bytes = ufid_no_owner.as_bytes();

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
