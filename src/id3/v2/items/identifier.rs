use crate::error::{Id3v2Error, Id3v2ErrorKind};
use crate::util::text::{decode_text, encode_text};
use crate::{Result, TextEncoding};

use std::hash::{Hash, Hasher};
use std::io::Read;

/// An `ID3v2` unique file identifier frame (UFID).
#[derive(Clone, Debug, Eq)]
pub struct UniqueFileIdentifierFrame {
	/// The non-empty owner of the identifier.
	pub owner: String,
	/// The binary payload with up to 64 bytes of data.
	pub identifier: Vec<u8>,
}

impl UniqueFileIdentifierFrame {
	/// Decode the frame contents from bytes
	///
	/// # Errors
	///
	/// Owner is missing or improperly encoded
	pub fn parse<R>(reader: &mut R) -> Result<Option<Self>>
	where
		R: Read,
	{
		let Some(owner) = decode_text(reader, TextEncoding::Latin1, true)?.text_or_none() else {
			return Err(Id3v2Error::new(Id3v2ErrorKind::MissingUfidOwner).into());
		};

		let mut identifier = Vec::new();
		reader.read_to_end(&mut identifier)?;

		Ok(Some(Self { owner, identifier }))
	}

	/// Encode the frame contents as bytes
	pub fn as_bytes(&self) -> Vec<u8> {
		let Self { owner, identifier } = self;

		let mut content = Vec::with_capacity(owner.len() + 1 + identifier.len());
		content.extend(encode_text(owner.as_str(), TextEncoding::Latin1, true));
		content.extend_from_slice(identifier);

		content
	}
}

impl PartialEq for UniqueFileIdentifierFrame {
	fn eq(&self, other: &Self) -> bool {
		self.owner == other.owner
	}
}

impl Hash for UniqueFileIdentifierFrame {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.owner.hash(state);
	}
}
