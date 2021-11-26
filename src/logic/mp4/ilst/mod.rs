pub(in crate::logic::mp4) mod read;
pub(in crate::logic) mod write;

use crate::types::item::{ItemKey, ItemValue, TagItem};
use crate::types::picture::Picture;
use crate::types::tag::{Tag, TagType};

use std::convert::TryInto;

#[derive(Default)]
/// An Mp4
pub struct Ilst {
	pub(crate) atoms: Vec<Atom>,
}

impl From<Ilst> for Tag {
	fn from(input: Ilst) -> Self {
		let mut tag = Self::new(TagType::Mp4Atom);

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
				&TagType::Mp4Atom,
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

impl From<Tag> for Ilst {
	fn from(input: Tag) -> Self {
		let mut ilst = Self::default();

		for item in input.items {
			if let Some(ident) = item_key_to_ident(item.key()).map(|k| k.into()) {
				let data = match item.item_value {
					ItemValue::Text(text) => AtomData::UTF8(text),
					_ => continue,
				};

				ilst.atoms.push(Atom { ident, data })
			}
		}

		ilst
	}
}

pub struct Atom {
	ident: AtomIdent,
	data: AtomData,
}

#[derive(Eq, PartialEq)]
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
	///
	/// * `mean`: A string using a reverse DNS naming convention
	/// * `name`: A string identifying the atom
	Freeform { mean: String, name: String },
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
pub enum AtomData {
	UTF8(String),
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
		code: u32,
		data: Vec<u8>,
	},
}

pub(crate) struct IlstRef<'a> {
	atoms: Box<dyn Iterator<Item = AtomRef<'a>> + 'a>,
}

pub(crate) struct AtomRef<'a> {
	ident: AtomIdentRef<'a>,
	data: AtomDataRef<'a>,
}

impl<'a> Into<AtomRef<'a>> for &'a Atom {
	fn into(self) -> AtomRef<'a> {
		AtomRef {
			ident: (&self.ident).into(),
			data: (&self.data).into(),
		}
	}
}

pub(crate) enum AtomIdentRef<'a> {
	Fourcc([u8; 4]),
	Freeform { mean: &'a str, name: &'a str },
}

impl<'a> Into<AtomIdentRef<'a>> for &'a AtomIdent {
	fn into(self) -> AtomIdentRef<'a> {
		match self {
			AtomIdent::Fourcc(fourcc) => AtomIdentRef::Fourcc(*fourcc),
			AtomIdent::Freeform { mean, name } => AtomIdentRef::Freeform { mean, name },
		}
	}
}

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

pub(crate) enum AtomDataRef<'a> {
	UTF8(&'a str),
	UTF16(&'a str),
	Picture(&'a Picture),
	SignedInteger(i32),
	UnsignedInteger(u32),
	Unknown { code: u32, data: &'a [u8] },
}

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

impl<'a> Into<IlstRef<'a>> for &'a Ilst {
	fn into(self) -> IlstRef<'a> {
		IlstRef {
			atoms: Box::new(self.atoms.iter().map(|a| a.into())),
		}
	}
}

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

fn item_key_to_ident(key: &ItemKey) -> Option<AtomIdentRef> {
	key.map_key(&TagType::Mp4Atom, true).and_then(|ident| {
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
