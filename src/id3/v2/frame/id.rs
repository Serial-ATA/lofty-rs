use crate::error::{Id3v2Error, Id3v2ErrorKind, LoftyError, Result};
use crate::types::item::ItemKey;
use crate::types::tag::TagType;

#[derive(PartialEq, Clone, Debug, Eq, Hash)]
/// An `ID3v2` frame ID
pub enum FrameID {
	/// A valid `ID3v2.3/4` frame
	Valid(String),
	/// When an `ID3v2.2` key couldn't be upgraded
	///
	/// This **will not** be written. It is up to the user to upgrade and store the key as [`Id3v2Frame::Valid`](Self::Valid).
	///
	/// The entire frame is stored as [`ItemValue::Binary`](crate::ItemValue::Binary).
	Outdated(String),
}

impl FrameID {
	/// Attempts to create a `FrameID` from an ID string
	///
	/// # Errors
	///
	/// * `id` contains invalid characters (must be 'A'..='Z' and '0'..='9')
	/// * `id` is an invalid length (must be 3 or 4)
	pub fn new(id: &str) -> Result<Self> {
		for c in id.chars() {
			if !('A'..='Z').contains(&c) && !('0'..='9').contains(&c) {
				return Err(Id3v2Error::new(Id3v2ErrorKind::BadFrameID).into());
			}
		}

		match id.len() {
			3 => Ok(FrameID::Outdated(id.to_string())),
			4 => Ok(FrameID::Valid(id.to_string())),
			_ => Err(Id3v2Error::new(Id3v2ErrorKind::BadFrameID).into()),
		}
	}

	/// Extracts the string from the ID
	pub fn as_str(&self) -> &str {
		match self {
			FrameID::Valid(v) | FrameID::Outdated(v) => v.as_str(),
		}
	}
}

impl TryFrom<ItemKey> for FrameID {
	type Error = LoftyError;

	fn try_from(value: ItemKey) -> std::prelude::rust_2015::Result<Self, Self::Error> {
		match value {
			ItemKey::Unknown(unknown)
				if unknown.len() == 4
					&& unknown
						.chars()
						.all(|c| ('A'..='Z').contains(&c) || ('0'..='9').contains(&c)) =>
			{
				Ok(Self::Valid(unknown))
			},
			k => k.map_key(TagType::Id3v2, false).map_or(
				Err(Id3v2Error::new(Id3v2ErrorKind::BadFrameID).into()),
				|id| Ok(Self::Valid(id.to_string())),
			),
		}
	}
}
