pub(crate) mod item;
pub(in crate::logic) mod read;
pub(in crate::logic) mod write;

use crate::error::Result;
use crate::logic::ape::tag::item::{ApeItem, ApeItemRef};
use crate::types::item::{ItemKey, ItemValue, TagItem};
use crate::types::tag::{Tag, TagType};

use std::convert::TryInto;
use std::fs::File;
use std::io::{Read, Seek};

#[derive(Default, Debug, PartialEq)]
/// An `APE` tag
///
/// ## Item storage
///
/// `APE` isn't a very strict format. An [`ApeItem`] only restricted by its name, meaning it can use
/// a normal [`ItemValue`](crate::ItemValue) unlike other formats.
///
/// Pictures are stored as [`ItemValue::Binary`](crate::ItemValue::Binary), and can be converted with
/// [`Picture::from_ape_bytes`](crate::Picture::from_ape_bytes). For the appropriate item keys, see
/// [APE_PICTURE_TYPES](crate::ape::APE_PICTURE_TYPES).
///
/// ## Conversions
///
/// ### From `Tag`
///
/// When converting pictures, any of type [`PictureType::Undefined`](crate::PictureType::Undefined) will be discarded.
/// For items, see [ApeItem::new].
pub struct ApeTag {
	/// Whether or not to mark the tag as read only
	pub read_only: bool,
	pub(super) items: Vec<ApeItem>,
}

impl ApeTag {
	/// Get an [`ApeItem`] by key
	///
	/// NOTE: While `APE` items are supposed to be case-sensitive,
	/// this rule is rarely followed, so this will ignore case when searching.
	pub fn get_key(&self, key: &str) -> Option<&ApeItem> {
		self.items
			.iter()
			.find(|i| i.key().eq_ignore_ascii_case(key))
	}

	/// Insert an [`ApeItem`]
	///
	/// This will remove any item with the same key prior to insertion
	pub fn insert(&mut self, value: ApeItem) {
		self.remove_key(value.key());
		self.items.push(value);
	}

	/// Remove an [`ApeItem`] by key
	///
	/// NOTE: Like [`ApeTag::get_key`], this is not case-sensitive
	pub fn remove_key(&mut self, key: &str) {
		self.items
			.iter()
			.position(|i| i.key().eq_ignore_ascii_case(key))
			.map(|p| self.items.remove(p));
	}

	/// Returns all of the tag's items
	pub fn items(&self) -> &[ApeItem] {
		&self.items
	}
}

impl ApeTag {
	#[allow(clippy::missing_errors_doc)]
	/// Parses an [`ApeTag`] from a reader
	///
	/// NOTE: This is **NOT** for reading from a file.
	/// This is used internally, and expects the `APE` preamble to have been skipped.
	pub fn read_from<R>(reader: &mut R) -> Result<Self>
	where
		R: Read + Seek,
	{
		Ok(read::read_ape_tag(reader, false)?.0)
	}

	/// Write an `APE` tag to a file
	///
	/// # Errors
	///
	/// * Attempting to write the tag to a format that does not support it
	/// * An existing tag has an invalid size
	pub fn write_to(&self, file: &mut File) -> Result<()> {
		Into::<ApeTagRef>::into(self).write_to(file)
	}
}

impl From<ApeTag> for Tag {
	fn from(input: ApeTag) -> Self {
		let mut tag = Tag::new(TagType::Ape);

		for item in input.items {
			let item = TagItem::new(ItemKey::from_key(&TagType::Ape, &*item.key), item.value);

			tag.insert_item_unchecked(item)
		}

		tag
	}
}

impl From<Tag> for ApeTag {
	fn from(input: Tag) -> Self {
		let mut ape_tag = Self::default();

		for item in input.items {
			if let Ok(ape_item) = item.try_into() {
				ape_tag.insert(ape_item)
			}
		}

		for pic in input.pictures {
			if let Some(key) = pic.pic_type.as_ape_key() {
				if let Ok(item) =
					ApeItem::new(key.to_string(), ItemValue::Binary(pic.as_ape_bytes()))
				{
					ape_tag.insert(item)
				}
			}
		}

		ape_tag
	}
}

pub(in crate::logic) struct ApeTagRef<'a> {
	read_only: bool,
	pub(super) items: Box<dyn Iterator<Item = ApeItemRef<'a>> + 'a>,
}

impl<'a> ApeTagRef<'a> {
	pub(crate) fn write_to(&mut self, file: &mut File) -> Result<()> {
		write::write_to(file, self)
	}
}

impl<'a> Into<ApeTagRef<'a>> for &'a Tag {
	fn into(self) -> ApeTagRef<'a> {
		ApeTagRef {
			read_only: false,
			items: Box::new(self.items.iter().filter_map(|i| {
				i.key().map_key(&TagType::Ape, true).map(|key| ApeItemRef {
					read_only: false,
					key,
					value: (&i.item_value).into(),
				})
			})),
		}
	}
}

impl<'a> Into<ApeTagRef<'a>> for &'a ApeTag {
	fn into(self) -> ApeTagRef<'a> {
		ApeTagRef {
			read_only: self.read_only,
			items: Box::new(self.items.iter().map(Into::into)),
		}
	}
}
