pub(super) mod parse;

use crate::id3::v2::FrameFlags;
use crate::id3::v2::error::FrameParseError;

use std::borrow::Cow;

/// An ID3v2 frame header
///
/// These are rarely constructed by hand. Usually they are created in the background
/// when making a new [`Frame`](crate::id3::v2::Frame).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FrameHeader<'a> {
	pub(crate) id: FrameId<'a>,
	/// The frame's flags
	pub flags: FrameFlags,
}

impl<'a> FrameHeader<'a> {
	/// Create a new [`FrameHeader`]
	///
	/// NOTE: Once the header is created, the ID becomes immutable.
	pub const fn new(id: FrameId<'a>, flags: FrameFlags) -> Self {
		Self { id, flags }
	}

	/// Get the ID of the frame
	pub const fn id(&'a self) -> &'a FrameId<'a> {
		&self.id
	}
}

impl FrameHeader<'static> {
	pub(crate) fn downgrade(&self) -> FrameHeader<'_> {
		FrameHeader {
			id: self.id.downgrade(),
			flags: self.flags,
		}
	}
}

/// Errors that can occur while converting a string to a [`FrameId`]
#[derive(Debug)]
pub struct FrameIdParseError {
	id_bytes: Vec<u8>,
}

impl core::fmt::Display for FrameIdParseError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "failed to parse a frame ID: 0x{:x?}", self.id_bytes)
	}
}

impl core::error::Error for FrameIdParseError {}

impl From<FrameIdParseError> for FrameParseError {
	fn from(input: FrameIdParseError) -> Self {
		FrameParseError::new(None, Box::new(input))
	}
}

/// An `ID3v2` frame ID
///
/// ⚠ WARNING ⚠: Be very careful when constructing this by hand. It is recommended to use [`FrameId::new`].
#[derive(PartialEq, Clone, Debug, Eq, Hash)]
pub enum FrameId<'a> {
	/// A valid `ID3v2.3/4` frame
	Valid(Cow<'a, str>),
	/// When an `ID3v2.2` key couldn't be upgraded
	///
	/// This **will not** be written. It is up to the user to upgrade and store the key as [`Id3v2Frame::Valid`](Self::Valid).
	///
	/// The entire frame is stored as [`ItemValue::Binary`](crate::tag::ItemValue::Binary).
	Outdated(Cow<'a, str>),
}

impl<'a> FrameId<'a> {
	/// Attempts to create a `FrameId` from an ID string
	///
	/// NOTE: This will not upgrade IDs.
	///
	/// # Errors
	///
	/// * `id` contains invalid characters (must be 'A'..='Z' and '0'..='9')
	/// * `id` is an invalid length (must be 3 or 4)
	pub fn new<I>(id: I) -> Result<Self, FrameIdParseError>
	where
		I: Into<Cow<'a, str>>,
	{
		Self::new_cow(id.into())
	}

	// Split from generic, public method to avoid code bloat by monomorphization.
	pub(in crate::id3::v2::frame) fn new_cow(id: Cow<'a, str>) -> Result<Self, FrameIdParseError> {
		Self::verify_id(&id)?;

		match id.len() {
			3 => Ok(FrameId::Outdated(id)),
			4 => Ok(FrameId::Valid(id)),
			_ => Err(FrameIdParseError {
				id_bytes: id.as_bytes().to_owned(),
			}),
		}
	}

	/// Whether this frame ID represents an outdated (ID3v2.2) ID
	///
	/// Note that frames with ID3v2.2 IDs *must* be upgraded to a 4-character ID3v2.3/4 ID in order to be
	/// written, otherwise they will be discarded.
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::id3::v2::FrameId;
	///
	/// # fn main() -> Result<(), lofty::id3::v2::error::FrameIdParseError> {
	/// let id_valid = FrameId::new("TPE1")?;
	/// assert!(!id_valid.is_outdated());
	///
	/// let id_outdated = FrameId::new("TP1")?;
	/// assert!(id_outdated.is_outdated());
	/// # Ok(()) }
	/// ```
	pub fn is_outdated(&self) -> bool {
		matches!(self, FrameId::Outdated(_))
	}

	/// Whether this frame ID represents a valid (ID3v2.3 or ID3v2.4) ID
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::id3::v2::FrameId;
	///
	/// # fn main() -> Result<(), lofty::id3::v2::error::FrameIdParseError> {
	/// let id_valid = FrameId::new("TPE1")?;
	/// assert!(id_valid.is_valid());
	///
	/// let id_outdated = FrameId::new("TP1")?;
	/// assert!(!id_outdated.is_valid());
	/// # Ok(()) }
	/// ```
	pub fn is_valid(&self) -> bool {
		matches!(self, FrameId::Valid(_))
	}

	/// Extracts the string from the ID
	pub fn as_str(&self) -> &str {
		match self {
			FrameId::Valid(v) | FrameId::Outdated(v) => v,
		}
	}

	pub(in crate::id3::v2::frame) fn verify_id(id_str: &str) -> Result<(), FrameIdParseError> {
		for c in id_str.chars() {
			if !c.is_ascii_uppercase() && !c.is_ascii_digit() {
				return Err(FrameIdParseError {
					id_bytes: id_str.as_bytes().to_vec(),
				});
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

	/// Creates an owned clone of the ID
	pub fn to_owned(&self) -> FrameId<'static> {
		match self {
			Self::Valid(inner) => FrameId::Valid(Cow::Owned(String::from(&**inner))),
			Self::Outdated(inner) => FrameId::Outdated(Cow::Owned(String::from(&**inner))),
		}
	}

	/// Obtains an owned instance
	pub fn into_owned(self) -> FrameId<'static> {
		match self {
			Self::Valid(inner) => FrameId::Valid(Cow::Owned(inner.into_owned())),
			Self::Outdated(inner) => FrameId::Outdated(Cow::Owned(inner.into_owned())),
		}
	}

	/// Consumes the [`FrameId`], returning the inner value
	pub fn into_inner(self) -> Cow<'a, str> {
		match self {
			FrameId::Valid(v) | FrameId::Outdated(v) => v,
		}
	}
}

impl FrameId<'static> {
	pub(crate) fn downgrade(&self) -> FrameId<'_> {
		match self {
			FrameId::Valid(id) => FrameId::Valid(Cow::Borrowed(&**id)),
			FrameId::Outdated(id) => FrameId::Outdated(Cow::Borrowed(&**id)),
		}
	}
}

impl core::fmt::Display for FrameId<'_> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.as_str())
	}
}

impl<'a> Into<Cow<'a, str>> for FrameId<'a> {
	fn into(self) -> Cow<'a, str> {
		self.into_inner()
	}
}
