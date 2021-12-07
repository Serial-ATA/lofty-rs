pub(in crate::logic::mp4) mod read;
pub(in crate::logic) mod write;

use crate::error::Result;
use crate::types::item::{ItemKey, ItemValue, TagItem};
use crate::types::picture::{Picture, PictureType};
use crate::types::tag::{Tag, TagType};

use std::convert::TryInto;
use std::fs::File;
use std::io::Read;

#[cfg(feature = "mp4_ilst")]
#[derive(Default, PartialEq, Debug)]
/// An MP4 ilst atom
///
/// ## Supported file types
///
/// * [`FileType::MP4`](crate::FileType::MP4)
///
/// ## Pictures
///
/// Unlike other formats, ilst does not store a [`PictureType`]. All pictures will have
/// [PictureType::Other].
///
/// ## Conversions
///
/// ### To `Tag`
///
/// When converting to [`Tag`], only atoms with a value of [AtomData::UTF8] and [AtomData::UTF16], as
/// well as pictures, will be preserved.
///
/// Do note, all pictures will be [PictureType::Other](crate::PictureType::Other)
///
/// ### From `Tag`
///
/// When converting from [`Tag`], only items with a value of [`ItemValue::Text`](crate::ItemValue::Text), as
/// well as pictures, will be preserved
pub struct Ilst {
	pub(crate) atoms: Vec<Atom>,
}

impl Ilst {
	/// Get an item by its [`AtomIdent`]
	pub fn atom(&self, ident: &AtomIdent) -> Option<&Atom> {
		self.atoms.iter().find(|a| &a.ident == ident)
	}

	/// Inserts an [`Atom`]
	pub fn insert_atom(&mut self, atom: Atom) {
		self.atoms.push(atom);
	}

	/// Inserts an [`Atom`], replacing any atom with the same [`AtomIdent`]
	pub fn replace_atom(&mut self, atom: Atom) {
		self.remove_atom(&atom.ident);
		self.atoms.push(atom);
	}

	/// Remove an atom by its [`AtomIdent`]
	pub fn remove_atom(&mut self, ident: &AtomIdent) {
		self.atoms
			.iter()
			.position(|a| &a.ident == ident)
			.map(|p| self.atoms.remove(p));
	}

	/// Returns all pictures
	pub fn pictures(&self) -> impl Iterator<Item = &Picture> {
		self.atoms.iter().filter_map(|a| {
			if a.ident() == &AtomIdent::Fourcc(*b"covr") {
				match a.data() {
					AtomData::Picture(pic) => Some(pic),
					_ => None,
				}
			} else {
				None
			}
		})
	}

	/// Inserts a picture
	pub fn insert_picture(&mut self, mut picture: Picture) {
		// This is just for correctness, it doesn't really matter.
		picture.pic_type = PictureType::Other;

		self.atoms.push(Atom {
			ident: AtomIdent::Fourcc(*b"covr"),
			data: AtomData::Picture(picture),
		})
	}

	/// Removes all pictures
	pub fn remove_pictures(&mut self) {
		self.atoms
			.retain(|a| !matches!(a.data(), AtomData::Picture(_)))
	}
}

impl Ilst {
	#[allow(clippy::missing_errors_doc)]
	/// Parses an [`Ilst`] from a reader
	///
	/// NOTE: This is **NOT** for reading from a file.
	/// This is used internally, and requires the length be provided.
	pub fn read_from<R>(reader: &mut R, len: u64) -> Result<Self>
	where
		R: Read,
	{
		read::parse_ilst(reader, len)
	}

	/// Writes the tag to a file
	///
	/// # Errors
	///
	/// * Attempting to write the tag to a format that does not support it
	pub fn write_to(&self, file: &mut File) -> Result<()> {
		Into::<IlstRef>::into(self).write_to(file)
	}
}

#[cfg(feature = "mp4_ilst")]
impl From<Ilst> for Tag {
	fn from(input: Ilst) -> Self {
		let mut tag = Self::new(TagType::Mp4Ilst);

		for atom in input.atoms {
			let value = match atom.data {
				AtomData::UTF8(text) | AtomData::UTF16(text) => ItemValue::Text(text),
				AtomData::Picture(pic) => {
					tag.pictures.push(pic);
					continue;
				},
				_ => continue,
			};

			let key = ItemKey::from_key(
				&TagType::Mp4Ilst,
				&match atom.ident {
					AtomIdent::Fourcc(fourcc) => {
						fourcc.iter().map(|b| *b as char).collect::<String>()
					},
					AtomIdent::Freeform { mean, name } => {
						format!("----:{}:{}", mean, name)
					},
				},
			);

			tag.items.push(TagItem::new(key, value));
		}

		tag
	}
}

#[cfg(feature = "mp4_ilst")]
impl From<Tag> for Ilst {
	fn from(input: Tag) -> Self {
		let mut ilst = Self::default();

		for item in input.items {
			if let Some(ident) = item_key_to_ident(item.key()).map(Into::into) {
				let data = match item.item_value {
					ItemValue::Text(text) => AtomData::UTF8(text),
					_ => continue,
				};

				ilst.atoms.push(Atom { ident, data });
			}
		}

		for mut picture in input.pictures {
			// Just for correctness, since we can't actually
			// assign a picture type in this format
			picture.pic_type = PictureType::Other;

			ilst.atoms.push(Atom {
				ident: AtomIdent::Fourcc([b'c', b'o', b'v', b'r']),
				data: AtomData::Picture(picture),
			})
		}

		ilst
	}
}

#[cfg(feature = "mp4_ilst")]
#[derive(Debug, PartialEq)]
/// Represents an `MP4` atom
pub struct Atom {
	ident: AtomIdent,
	data: AtomData,
}

impl Atom {
	/// Create a new [`Atom`]
	pub fn new(ident: AtomIdent, data: AtomData) -> Self {
		Self { ident, data }
	}

	/// Returns the atom's [`AtomIdent`]
	pub fn ident(&self) -> &AtomIdent {
		&self.ident
	}

	/// Returns the atom's [`AtomData`]
	pub fn data(&self) -> &AtomData {
		&self.data
	}
}

#[derive(Eq, PartialEq, Debug)]
/// Represents an `MP4` atom identifier
pub enum AtomIdent {
	/// A four byte identifier
	///
	/// Many FOURCCs start with `0xA9` (©), and should be a UTF-8 string.
	Fourcc([u8; 4]),
	/// A freeform identifier
	///
	/// # Example
	///
	/// ```text
	/// ----:com.apple.iTunes:SUBTITLE
	/// ─┬── ────────┬─────── ───┬────
	///  ╰freeform identifier    ╰name
	///              |
	///              ╰mean
	/// ```
	Freeform {
		/// A string using a reverse DNS naming convention
		mean: String,
		/// A string identifying the atom
		name: String,
	},
}

#[cfg(feature = "mp4_ilst")]
#[derive(Debug, PartialEq)]
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
	SignedInteger(i32),
	/// A big endian unsigned integer (1-4 bytes)
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

#[cfg(feature = "mp4_ilst")]
pub(crate) struct IlstRef<'a> {
	atoms: Box<dyn Iterator<Item = AtomRef<'a>> + 'a>,
}

impl<'a> IlstRef<'a> {
	pub(in crate::logic) fn write_to(&mut self, file: &mut File) -> Result<()> {
		write::write_to(file, self)
	}
}

#[cfg(feature = "mp4_ilst")]
pub(crate) struct AtomRef<'a> {
	ident: AtomIdentRef<'a>,
	data: AtomDataRef<'a>,
}

#[cfg(feature = "mp4_ilst")]
impl<'a> Into<AtomRef<'a>> for &'a Atom {
	fn into(self) -> AtomRef<'a> {
		AtomRef {
			ident: (&self.ident).into(),
			data: (&self.data).into(),
		}
	}
}

#[cfg(feature = "mp4_ilst")]
pub(crate) enum AtomIdentRef<'a> {
	Fourcc([u8; 4]),
	Freeform { mean: &'a str, name: &'a str },
}

#[cfg(feature = "mp4_ilst")]
impl<'a> Into<AtomIdentRef<'a>> for &'a AtomIdent {
	fn into(self) -> AtomIdentRef<'a> {
		match self {
			AtomIdent::Fourcc(fourcc) => AtomIdentRef::Fourcc(*fourcc),
			AtomIdent::Freeform { mean, name } => AtomIdentRef::Freeform { mean, name },
		}
	}
}

#[cfg(feature = "mp4_ilst")]
impl<'a> From<AtomIdentRef<'a>> for AtomIdent {
	fn from(input: AtomIdentRef<'a>) -> Self {
		match input {
			AtomIdentRef::Fourcc(fourcc) => AtomIdent::Fourcc(fourcc),
			AtomIdentRef::Freeform { mean, name } => AtomIdent::Freeform {
				mean: mean.to_string(),
				name: name.to_string(),
			},
		}
	}
}

#[cfg(feature = "mp4_ilst")]
pub(crate) enum AtomDataRef<'a> {
	UTF8(&'a str),
	UTF16(&'a str),
	Picture(&'a Picture),
	SignedInteger(i32),
	UnsignedInteger(u32),
	Unknown { code: u32, data: &'a [u8] },
}

#[cfg(feature = "mp4_ilst")]
impl<'a> Into<AtomDataRef<'a>> for &'a AtomData {
	fn into(self) -> AtomDataRef<'a> {
		match self {
			AtomData::UTF8(utf8) => AtomDataRef::UTF8(utf8),
			AtomData::UTF16(utf16) => AtomDataRef::UTF16(utf16),
			AtomData::Picture(pic) => AtomDataRef::Picture(pic),
			AtomData::SignedInteger(int) => AtomDataRef::SignedInteger(*int),
			AtomData::UnsignedInteger(uint) => AtomDataRef::UnsignedInteger(*uint),
			AtomData::Unknown { code, data } => AtomDataRef::Unknown { code: *code, data },
		}
	}
}

#[cfg(feature = "mp4_ilst")]
impl<'a> Into<IlstRef<'a>> for &'a Ilst {
	fn into(self) -> IlstRef<'a> {
		IlstRef {
			atoms: Box::new(self.atoms.iter().map(Into::into)),
		}
	}
}

#[cfg(feature = "mp4_ilst")]
impl<'a> Into<IlstRef<'a>> for &'a Tag {
	fn into(self) -> IlstRef<'a> {
		let iter =
			self.items
				.iter()
				.filter_map(|i| match (item_key_to_ident(i.key()), i.value()) {
					(Some(ident), ItemValue::Text(text)) => Some(AtomRef {
						ident,
						data: AtomDataRef::UTF8(text),
					}),
					_ => None,
				});

		IlstRef {
			atoms: Box::new(iter),
		}
	}
}

#[cfg(feature = "mp4_ilst")]
fn item_key_to_ident(key: &ItemKey) -> Option<AtomIdentRef> {
	key.map_key(&TagType::Mp4Ilst, true).and_then(|ident| {
		if ident.starts_with("----") {
			let mut split = ident.split(':');

			split.next();

			let mean = split.next();
			let name = split.next();

			if let (Some(mean), Some(name)) = (mean, name) {
				Some(AtomIdentRef::Freeform { mean, name })
			} else {
				None
			}
		} else {
			let fourcc = ident.chars().map(|c| c as u8).collect::<Vec<_>>();

			if let Ok(fourcc) = TryInto::<[u8; 4]>::try_into(fourcc) {
				Some(AtomIdentRef::Fourcc(fourcc))
			} else {
				None
			}
		}
	})
}
