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
	pub fn ident(&self) -> AtomIdent<'_> {
		self.ident.as_borrowed()
	}

	/// Returns the atom's [`AtomData`]
	pub fn data(&self) -> impl Iterator<Item = &AtomData> {
		(&self.data).into_iter()
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

	// Used internally, has no correctness checks
	pub(crate) fn unknown_implicit(ident: AtomIdent<'_>, data: Vec<u8>) -> Self {
		Self {
			ident: ident.into_owned(),
			data: AtomDataStorage::Single(AtomData::Unknown { code: 0, data }),
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
/// See the list of [well-known data types](https://developer.apple.com/library/archive/documentation/QuickTime/QTFF/Metadata/Metadata.html#//apple_ref/doc/uid/TP40000939-CH1-SW34)
/// for codes.
/// * There are only two variants for integers, which
/// will come from codes `21` and `22`. All other integer
/// types will be stored as [`AtomData::Unknown`], refer
/// to the link above for codes.
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
	///       is `SignedInteger`.
	Bool(bool),
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

/// The parental advisory rating
///
/// See also:
/// <https://docs.mp3tag.de/mapping/#itunesadvisory>
/// <https://exiftool.org/TagNames/QuickTime.html>
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdvisoryRating {
	/// *Inoffensive*/*None* (0)
	Inoffensive,
	/// *Explicit* (1 or 4)
	///
	/// In the past Apple used the value 4 for explicit content
	/// that has later been replaced by 1. Both values are considered
	/// as valid when reading but only the newer value 1 is written.
	Explicit,
	/// *Clean*/*Edited* (2)
	Clean,
}

impl AdvisoryRating {
	/// Returns the rating as it appears in the `rtng` atom
	pub fn as_u8(&self) -> u8 {
		match self {
			AdvisoryRating::Inoffensive => 0,
			AdvisoryRating::Explicit => 1,
			AdvisoryRating::Clean => 2,
		}
	}
}

impl TryFrom<u8> for AdvisoryRating {
	type Error = u8;

	fn try_from(input: u8) -> Result<Self, Self::Error> {
		match input {
			0 => Ok(Self::Inoffensive),
			1 | 4 => Ok(Self::Explicit),
			2 => Ok(Self::Clean),
			value => Err(value),
		}
	}
}
