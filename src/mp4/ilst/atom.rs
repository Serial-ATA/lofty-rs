use crate::mp4::AtomIdent;
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
	pub(super) fn take_first(self) -> AtomData {
		match self {
			AtomDataStorage::Single(val) => val,
			AtomDataStorage::Multiple(mut data) => data.swap_remove(0),
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
				}

				self.idx += 1;
				Some(&data[self.idx])
			},
			_ => None,
		}
	}
}

#[derive(PartialEq, Clone)]
/// Represents an `MP4` atom
pub struct Atom {
	pub(crate) ident: AtomIdent,
	pub(super) data: AtomDataStorage,
}

impl Atom {
	/// Create a new [`Atom`]
	pub fn new(ident: AtomIdent, data: AtomData) -> Self {
		Self {
			ident,
			data: AtomDataStorage::Single(data),
		}
	}

	/// Create a new [`Atom`] from a collection of [`AtomData`]s
	///
	/// This will return `None` if `data` is empty, as empty atoms are useless.
	pub fn from_collection(ident: AtomIdent, mut data: Vec<AtomData>) -> Option<Self> {
		let data = match data.len() {
			0 => return None,
			1 => AtomDataStorage::Single(data.swap_remove(0)),
			_ => AtomDataStorage::Multiple(data),
		};

		Some(Self { ident, data })
	}

	/// Returns the atom's [`AtomIdent`]
	pub fn ident(&self) -> &AtomIdent {
		&self.ident
	}

	/// Returns the atom's [`AtomData`]
	// TODO: Do this properly to return all values
	pub fn data(&self) -> &AtomData {
		match &self.data {
			AtomDataStorage::Single(val) => val,
			// There must be at least 1 element in here
			AtomDataStorage::Multiple(data) => &data[0],
		}
	}

	// TODO: push_data
}

impl Debug for Atom {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Atom")
			.field("ident", &self.ident)
			.field("data", &self.data)
			.finish()
	}
}

// TODO: Bool variant for the various flag atoms?
#[derive(Debug, PartialEq, Clone)]
/// The data of an atom
///
/// NOTES:
///
/// * This only covers the most common data types.
/// See the list of [well-known data types](https://developer.apple.com/library/archive/documentation/QuickTime/QTFF/Metadata/Metadata.html#//apple_ref/doc/uid/TP40000939-CH1-SW34)
/// for codes.
/// * There are only two variants for integers, which
/// will come from codes `21` and `22`. All other integer
/// types will be stored as [`AtomData::Unknown`], refer
/// to the link above for codes.
pub enum AtomData {
	/// A UTF-8 encoded string
	UTF8(String),
	/// A UTf-16 encoded string
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
	/// Unknown data
	///
	/// Due to the number of possible types, there are many
	/// **specified** types that are going to fall into this
	/// variant.
	Unknown {
		/// The code, or type of the item
		code: u32,
		/// The binary data of the atom
		data: Vec<u8>,
	},
}

#[derive(Clone, Copy, Debug, PartialEq)]
/// The parental advisory rating
pub enum AdvisoryRating {
	/// A rating of 0
	Inoffensive,
	/// A rating of 2
	Clean,
	/// A rating of (1 || > 2)
	Explicit,
}

impl AdvisoryRating {
	/// Returns the rating as it appears in the `rtng` atom
	pub fn as_u8(&self) -> u8 {
		match self {
			AdvisoryRating::Inoffensive => 0,
			AdvisoryRating::Clean => 2,
			AdvisoryRating::Explicit => 4,
		}
	}
}

impl From<u8> for AdvisoryRating {
	fn from(input: u8) -> Self {
		match input {
			0 => Self::Inoffensive,
			2 => Self::Clean,
			_ => Self::Explicit,
		}
	}
}
