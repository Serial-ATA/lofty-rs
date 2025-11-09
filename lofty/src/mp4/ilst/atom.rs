use crate::error::Result;
use crate::macros::err;
use crate::mp4::AtomIdent;
use crate::mp4::ilst::data_type::DataType;
use crate::picture::Picture;

use std::fmt::{Debug, Formatter};

// Atoms with multiple values aren't all that common,
// so there's no need to create a bunch of single-element Vecs
#[derive(PartialEq, Clone)]
pub(super) enum AtomDataStorage {
	Single(AtomData),
	Multiple(Vec<AtomData>),
}

impl AtomDataStorage {
	pub(super) fn first_mut(&mut self) -> &mut AtomData {
		match self {
			AtomDataStorage::Single(val) => val,
			AtomDataStorage::Multiple(data) => data.first_mut().expect("not empty"),
		}
	}

	pub(super) fn is_pictures(&self) -> bool {
		match self {
			AtomDataStorage::Single(v) => matches!(v, AtomData::Picture(_)),
			AtomDataStorage::Multiple(v) => v.iter().all(|p| matches!(p, AtomData::Picture(_))),
		}
	}

	pub(super) fn from_vec(mut v: Vec<AtomData>) -> Option<Self> {
		match v.len() {
			0 => None,
			1 => Some(AtomDataStorage::Single(v.remove(0))),
			_ => Some(AtomDataStorage::Multiple(v)),
		}
	}
}

impl Debug for AtomDataStorage {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match &self {
			AtomDataStorage::Single(v) => write!(f, "{:?}", v),
			AtomDataStorage::Multiple(v) => f.debug_list().entries(v.iter()).finish(),
		}
	}
}

impl IntoIterator for AtomDataStorage {
	type Item = AtomData;
	type IntoIter = std::vec::IntoIter<Self::Item>;

	fn into_iter(self) -> Self::IntoIter {
		match self {
			AtomDataStorage::Single(s) => vec![s].into_iter(),
			AtomDataStorage::Multiple(v) => v.into_iter(),
		}
	}
}

impl<'a> IntoIterator for &'a AtomDataStorage {
	type Item = &'a AtomData;
	type IntoIter = AtomDataStorageIter<'a>;

	fn into_iter(self) -> Self::IntoIter {
		let cap = match self {
			AtomDataStorage::Single(_) => 0,
			AtomDataStorage::Multiple(v) => v.len(),
		};

		Self::IntoIter {
			storage: Some(self),
			idx: 0,
			cap,
		}
	}
}

pub(super) struct AtomDataStorageIter<'a> {
	storage: Option<&'a AtomDataStorage>,
	idx: usize,
	cap: usize,
}

impl<'a> Iterator for AtomDataStorageIter<'a> {
	type Item = &'a AtomData;

	fn next(&mut self) -> Option<Self::Item> {
		match self.storage {
			Some(AtomDataStorage::Single(data)) => {
				self.storage = None;
				Some(data)
			},
			Some(AtomDataStorage::Multiple(data)) => {
				if self.idx == self.cap {
					self.storage = None;
					return None;
				}

				let ret = &data[self.idx];
				self.idx += 1;

				Some(ret)
			},
			_ => None,
		}
	}
}

/// Represents an `MP4` atom
#[derive(PartialEq, Clone)]
pub struct Atom<'a> {
	pub(crate) ident: AtomIdent<'a>,
	pub(super) data: AtomDataStorage,
}

impl<'a> Atom<'a> {
	/// Create a new [`Atom`]
	#[must_use]
	pub const fn new(ident: AtomIdent<'a>, data: AtomData) -> Self {
		Self {
			ident,
			data: AtomDataStorage::Single(data),
		}
	}

	/// Create a new [`Atom`] from a collection of [`AtomData`]s
	///
	/// This will return `None` if `data` is empty, as empty atoms are useless.
	pub fn from_collection(ident: AtomIdent<'a>, mut data: Vec<AtomData>) -> Option<Self> {
		let data = match data.len() {
			0 => return None,
			1 => AtomDataStorage::Single(data.swap_remove(0)),
			_ => AtomDataStorage::Multiple(data),
		};

		Some(Self { ident, data })
	}

	/// Returns the atom's [`AtomIdent`]
	pub fn ident(&self) -> &AtomIdent<'_> {
		&self.ident
	}

	/// Returns the atom's [`AtomData`]
	pub fn data(&self) -> impl Iterator<Item = &AtomData> {
		(&self.data).into_iter()
	}

	/// Consumes the atom, returning its [`AtomData`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::mp4::{Atom, AtomData, AtomIdent};
	///
	/// let atom = Atom::new(
	/// 	AtomIdent::Fourcc(*b"\x49ART"),
	/// 	AtomData::UTF8(String::from("Foo")),
	/// );
	/// assert_eq!(atom.into_data().count(), 1);
	/// ```
	pub fn into_data(self) -> impl Iterator<Item = AtomData> + use<> {
		self.data.into_iter()
	}

	/// Append a value to the atom
	pub fn push_data(&mut self, data: AtomData) {
		match self.data {
			AtomDataStorage::Single(ref s) => {
				self.data = AtomDataStorage::Multiple(vec![s.clone(), data])
			},
			AtomDataStorage::Multiple(ref mut m) => m.push(data),
		}
	}

	/// Merge the data of another atom into this one
	///
	/// NOTE: Both atoms must have the same identifier
	///
	/// # Errors
	///
	/// * `self.ident()` != `other.ident()`
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::mp4::{Atom, AtomData, AtomIdent};
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// // Create an artist atom
	/// let mut atom = Atom::new(
	/// 	AtomIdent::Fourcc(*b"\x49ART"),
	/// 	AtomData::UTF8(String::from("foo")),
	/// );
	///
	/// // Create a second and merge it into the first
	/// let atom2 = Atom::new(
	/// 	AtomIdent::Fourcc(*b"\x49ART"),
	/// 	AtomData::UTF8(String::from("bar")),
	/// );
	/// atom.merge(atom2)?;
	///
	/// // Our first atom now contains the second atom's content
	/// assert_eq!(atom.data().count(), 2);
	/// # Ok(()) }
	/// ```
	pub fn merge(&mut self, other: Atom<'_>) -> Result<()> {
		if self.ident != other.ident {
			err!(AtomMismatch);
		}

		for data in other.data {
			self.push_data(data)
		}

		Ok(())
	}

	// Used internally, has no correctness checks
	pub(crate) fn unknown_implicit(ident: AtomIdent<'_>, data: Vec<u8>) -> Self {
		Self {
			ident: ident.into_owned(),
			data: AtomDataStorage::Single(AtomData::Unknown {
				code: DataType::Reserved,
				data,
			}),
		}
	}

	pub(crate) fn text(ident: AtomIdent<'_>, data: String) -> Self {
		Self {
			ident: ident.into_owned(),
			data: AtomDataStorage::Single(AtomData::UTF8(data)),
		}
	}

	pub(crate) fn into_owned(self) -> Atom<'static> {
		let Self { ident, data } = self;
		Atom {
			ident: ident.into_owned(),
			data,
		}
	}
}

impl Debug for Atom<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Atom")
			.field("ident", &self.ident)
			.field("data", &self.data)
			.finish()
	}
}

/// The data of an atom
///
/// NOTES:
///
/// * This only covers the most common data types.
///   See the list of [DataType] for all known types.
/// * There are only two variants for integers, which
///   will come from codes `21` and `22`. All other integer
///   types will be stored as [`AtomData::Unknown`], refer
///   to the link above for codes.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AtomData {
	/// A UTF-8 encoded string
	UTF8(String),
	/// A UTF-16 encoded string
	UTF16(String),
	/// A JPEG, PNG, GIF *(Deprecated)*, or BMP image
	///
	/// The type is read from the picture itself
	Picture(Picture),
	/// A big endian signed integer (1-4 bytes)
	///
	/// NOTE:
	///
	/// This will shrink the integer when writing
	///
	/// 255 will be written as `[255]` rather than `[0, 0, 0, 255]`
	///
	/// This behavior may be unexpected, use [`AtomData::Unknown`] if unsure
	SignedInteger(i32),
	/// A big endian unsigned integer (1-4 bytes)
	///
	/// NOTE: See [`AtomData::SignedInteger`]
	UnsignedInteger(u32),
	/// A boolean value
	///
	/// NOTE: This isn't an official data type, but multiple flag atoms exist,
	///       so this makes them easier to represent. The *real* underlying type
	///       is [`Self::SignedInteger`].
	Bool(bool),
	/// Unknown data
	///
	/// Due to the number of possible types, there are many
	/// **specified** types that are going to fall into this
	/// variant. See [`DataType`] for a list of known types.
	Unknown {
		/// The code, or type of the item
		code: DataType,
		/// The binary data of the atom
		data: Vec<u8>,
	},
}

impl AtomData {
	/// Get the [`DataType`] of the atom
	///
	/// Note that for [`AtomData::Picture`], the type is determined by the picture's MIME type.
	/// If the MIME type is unknown (or unset), the data type will be [`DataType::Reserved`].
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::mp4::{AtomData, DataType};
	/// use lofty::picture::{MimeType, Picture, PictureType};
	///
	/// let data = AtomData::UTF8(String::from("foo"));
	/// assert_eq!(data.data_type(), DataType::Utf8);
	///
	/// let data = AtomData::SignedInteger(42);
	/// assert_eq!(data.data_type(), DataType::BeSignedInteger);
	///
	/// let data = AtomData::Picture(
	/// 	Picture::unchecked(Vec::new())
	/// 		.pic_type(PictureType::CoverFront)
	/// 		.mime_type(MimeType::Jpeg)
	/// 		.build(),
	/// );
	/// assert_eq!(data.data_type(), DataType::Jpeg);
	/// ```
	pub fn data_type(&self) -> DataType {
		match self {
			AtomData::UTF8(_) => DataType::Utf8,
			AtomData::UTF16(_) => DataType::Utf16,
			AtomData::SignedInteger(_) | AtomData::Bool(_) => DataType::BeSignedInteger,
			AtomData::UnsignedInteger(_) => DataType::BeUnsignedInteger,
			AtomData::Picture(p) => {
				let Some(mime) = p.mime_type() else {
					return DataType::Reserved;
				};

				DataType::from(mime)
			},
			AtomData::Unknown { code, .. } => *code,
		}
	}
}
