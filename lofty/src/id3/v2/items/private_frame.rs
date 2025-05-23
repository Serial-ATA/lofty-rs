use crate::error::Result;
use crate::id3::v2::{FrameFlags, FrameHeader, FrameId};
use crate::util::alloc::VecFallibleCapacity;
use crate::util::text::{TextDecodeOptions, TextEncoding, decode_text, encode_text};

use std::borrow::Cow;
use std::io::Read;

const FRAME_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("PRIV"));

/// An `ID3v2` private frame
///
/// This frame is used to contain information from a software producer that
/// its program uses and does not fit into the other frames.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct PrivateFrame<'a> {
	pub(crate) header: FrameHeader<'a>,
	/// A URL containing an email address, or a link to a location where an email can be found,
	/// that belongs to the organisation responsible for the frame
	pub owner: String,
	/// Binary data
	pub private_data: Vec<u8>,
}

impl PrivateFrame<'_> {
	/// Create a new [`PrivateFrame`]
	pub fn new(owner: String, private_data: Vec<u8>) -> Self {
		let header = FrameHeader::new(FRAME_ID, FrameFlags::default());
		Self {
			header,
			owner,
			private_data,
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

	/// Read an [`PrivateFrame`]
	///
	/// NOTE: This expects the frame header to have already been skipped
	///
	/// # Errors
	///
	/// * Failure to read from `reader`
	pub fn parse<R>(reader: &mut R, frame_flags: FrameFlags) -> Result<Option<Self>>
	where
		R: Read,
	{
		let Ok(owner) = decode_text(
			reader,
			TextDecodeOptions::new()
				.encoding(TextEncoding::Latin1)
				.terminated(true),
		) else {
			return Ok(None);
		};

		let owner = owner.content;

		let mut private_data = Vec::new();
		reader.read_to_end(&mut private_data)?;

		let header = FrameHeader::new(FRAME_ID, frame_flags);
		Ok(Some(PrivateFrame {
			header,
			owner,
			private_data,
		}))
	}

	/// Convert an [`PrivateFrame`] to a byte vec
	///
	/// # Errors
	///
	/// * The resulting [`Vec`] exceeds [`GlobalOptions::allocation_limit`](crate::config::GlobalOptions::allocation_limit)
	pub fn as_bytes(&self) -> Result<Vec<u8>> {
		let Self {
			owner,
			private_data,
			..
		} = self;

		let mut content = Vec::try_with_capacity_stable(owner.len() + private_data.len())?;
		content.extend(encode_text(owner.as_str(), TextEncoding::Latin1, true));
		content.extend_from_slice(private_data);

		Ok(content)
	}
}

#[cfg(test)]
mod tests {
	use crate::id3::v2::{FrameFlags, FrameHeader, PrivateFrame};

	fn expected() -> PrivateFrame<'static> {
		PrivateFrame {
			header: FrameHeader::new(super::FRAME_ID, FrameFlags::default()),
			owner: String::from("foo@bar.com"),
			private_data: String::from("some data").into_bytes(),
		}
	}

	#[test_log::test]
	fn priv_decode() {
		let cont = crate::tag::utils::test_utils::read_path("tests/tags/assets/id3v2/test.priv");

		let parsed_priv = PrivateFrame::parse(&mut &cont[..], FrameFlags::default())
			.unwrap()
			.unwrap();

		assert_eq!(parsed_priv, expected());
	}

	#[test_log::test]
	fn priv_encode() {
		let encoded = expected().as_bytes().unwrap();

		let expected_bytes =
			crate::tag::utils::test_utils::read_path("tests/tags/assets/id3v2/test.priv");

		assert_eq!(encoded, expected_bytes);
	}
}
