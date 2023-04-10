use crate::util::text::{encode_text, TextEncoding};

use std::hash::{Hash, Hasher};

/// An extended `ID3v2` URL frame
///
/// This is used in the `WXXX` frame, where the frames
/// are told apart by descriptions, rather than their [`FrameID`](crate::id3::v2::FrameID)s.
/// This means for each `ExtendedUrlFrame` in the tag, the description
/// must be unique.
#[derive(Clone, Debug, Eq)]
pub struct ExtendedUrlFrame {
	/// The encoding of the description and comment text
	pub encoding: TextEncoding,
	/// Unique content description
	pub description: String,
	/// The actual frame content
	pub content: String,
}

impl PartialEq for ExtendedUrlFrame {
	fn eq(&self, other: &Self) -> bool {
		self.description == other.description
	}
}

impl Hash for ExtendedUrlFrame {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.description.hash(state);
	}
}

impl ExtendedUrlFrame {
	/// Convert an [`ExtendedUrlFrame`] to a byte vec
	pub fn as_bytes(&self) -> Vec<u8> {
		let mut bytes = vec![self.encoding as u8];

		bytes.extend(encode_text(&self.description, self.encoding, true).iter());
		bytes.extend(encode_text(&self.content, self.encoding, false));

		bytes
	}
}
