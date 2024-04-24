use crate::error::Result;
use crate::util::text::{decode_text, encode_text, TextDecodeOptions, TextEncoding};

use std::io::Read;

/// An `ID3v2` private frame
///
/// This frame is used to contain information from a software producer that
/// its program uses and does not fit into the other frames.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct PrivateFrame {
	/// A URL containing an email address, or a link to a location where an email can be found,
	/// that belongs to the organisation responsible for the frame
	pub owner: String,
	/// Binary data
	pub private_data: Vec<u8>,
}

impl PrivateFrame {
	/// Read an [`PrivateFrame`]
	///
	/// NOTE: This expects the frame header to have already been skipped
	///
	/// # Errors
	///
	/// * Failure to read from `reader`
	pub fn parse<R>(reader: &mut R) -> Result<Option<Self>>
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

		Ok(Some(PrivateFrame {
			owner,
			private_data,
		}))
	}

	/// Convert an [`PrivateFrame`] to a byte vec
	pub fn as_bytes(&self) -> Vec<u8> {
		let Self {
			owner,
			private_data,
		} = self;

		let mut content = Vec::with_capacity(owner.len() + private_data.len());
		content.extend(encode_text(owner.as_str(), TextEncoding::Latin1, true));
		content.extend_from_slice(private_data);

		content
	}
}

#[cfg(test)]
mod tests {
	use crate::id3::v2::PrivateFrame;

	fn expected() -> PrivateFrame {
		PrivateFrame {
			owner: String::from("foo@bar.com"),
			private_data: String::from("some data").into_bytes(),
		}
	}

	#[test]
	fn priv_decode() {
		let cont = crate::tag::utils::test_utils::read_path("tests/tags/assets/id3v2/test.priv");

		let parsed_priv = PrivateFrame::parse(&mut &cont[..]).unwrap().unwrap();

		assert_eq!(parsed_priv, expected());
	}

	#[test]
	fn priv_encode() {
		let encoded = expected().as_bytes();

		let expected_bytes =
			crate::tag::utils::test_utils::read_path("tests/tags/assets/id3v2/test.priv");

		assert_eq!(encoded, expected_bytes);
	}
}
