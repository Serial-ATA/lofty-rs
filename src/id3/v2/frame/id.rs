use std::borrow::Cow;

use crate::error::{Id3v2Error, Id3v2ErrorKind, LoftyError, Result};
use crate::tag::item::ItemKey;
use crate::tag::TagType;

/// An `ID3v2` frame ID
#[derive(PartialEq, Clone, Debug, Eq, Hash)]
pub enum FrameID<'a> {
	/// A valid `ID3v2.3/4` frame
	Valid(Cow<'a, str>),
	/// When an `ID3v2.2` key couldn't be upgraded
	///
	/// This **will not** be written. It is up to the user to upgrade and store the key as [`Id3v2Frame::Valid`](Self::Valid).
	///
	/// The entire frame is stored as [`ItemValue::Binary`](crate::ItemValue::Binary).
	Outdated(Cow<'a, str>),
}

impl<'a> FrameID<'a> {
	/// Attempts to create a `FrameID` from an ID string
	///
	/// # Errors
	///
	/// * `id` contains invalid characters (must be 'A'..='Z' and '0'..='9')
	/// * `id` is an invalid length (must be 3 or 4)
	pub fn new<I>(id: I) -> Result<Self>
	where
		I: Into<Cow<'a, str>>,
	{
		Self::new_cow(id.into())
	}

	// Split from generic, public method to avoid code bloat by monomorphization.
	pub(super) fn new_cow(id: Cow<'a, str>) -> Result<Self> {
		Self::verify_id(&id)?;

		match id.len() {
			3 => Ok(FrameID::Outdated(id)),
			4 => Ok(FrameID::Valid(id)),
			_ => Err(Id3v2Error::new(Id3v2ErrorKind::BadFrameID).into()),
		}
	}

	/// Extracts the string from the ID
	pub fn as_str(&self) -> &str {
		match self {
			FrameID::Valid(v) | FrameID::Outdated(v) => v,
		}
	}

	pub(super) fn verify_id(id_str: &str) -> Result<()> {
		for c in id_str.chars() {
			if !c.is_ascii_uppercase() && !c.is_ascii_digit() {
				return Err(Id3v2Error::new(Id3v2ErrorKind::BadFrameID).into());
			}
		}

		Ok(())
	}

	/// Obtains a borrowed instance
	pub fn as_borrowed(&'a self) -> Self {
		match self {
			Self::Valid(inner) => Self::Valid(Cow::Borrowed(inner)),
			Self::Outdated(inner) => Self::Outdated(Cow::Borrowed(inner)),
		}
	}

	/// Obtains an owned instance
	pub fn into_owned(self) -> FrameID<'static> {
		match self {
			Self::Valid(inner) => FrameID::Valid(Cow::Owned(inner.into_owned())),
			Self::Outdated(inner) => FrameID::Outdated(Cow::Owned(inner.into_owned())),
		}
	}
}

impl<'a> TryFrom<&'a ItemKey> for FrameID<'a> {
	type Error = LoftyError;

	fn try_from(value: &'a ItemKey) -> std::prelude::rust_2015::Result<Self, Self::Error> {
		match value {
			ItemKey::Unknown(unknown) if unknown.len() == 4 => {
				Self::verify_id(unknown)?;
				Ok(Self::Valid(Cow::Borrowed(unknown)))
			},
			k => {
				if let Some(mapped) = k.map_key(TagType::ID3v2, false) {
					if mapped.len() == 4 {
						Self::verify_id(mapped)?;
						return Ok(Self::Valid(Cow::Borrowed(mapped)));
					}
				}

				Err(Id3v2Error::new(Id3v2ErrorKind::BadFrameID).into())
			},
		}
	}
}
