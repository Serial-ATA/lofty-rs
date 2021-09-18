use super::item::{ItemKey, ItemValue, TagItem};
use super::picture::{Picture, PictureType};
use crate::error::{LoftyError, Result};
#[cfg(feature = "id3v2_restrictions")]
use crate::logic::id3::v2::items::restrictions::TagRestrictions;
use crate::probe::Probe;

use std::fs::File;

#[cfg(feature = "quick_tag_accessors")]
use paste::paste;

#[cfg(feature = "quick_tag_accessors")]
macro_rules! common_items {
	($($item_key:ident => $name:tt),+) => {
		paste! {
			impl Tag {
				$(
					#[doc = "Gets the " $name]
					pub fn $name(&self) -> Option<&str> {
						if let Some(ItemValue::Text(txt)) = self.get_item_ref(&ItemKey::$item_key).map(TagItem::value) {
							return Some(&*txt)
						}

						None
					}

					#[doc = "Removes the " $name]
					pub fn [<remove_ $name>](&mut self) {
						self.retain(|i| i.item_key != ItemKey::$item_key)
					}

					#[doc = "Sets the " $name]
					pub fn [<set_ $name>](&mut self, value: String) {
						self.insert_item(TagItem::new(ItemKey::$item_key, ItemValue::Text(value)));
					}
				)+
			}
		}
	}
}

#[cfg(feature = "id3v2")]
#[derive(Default, Copy, Clone)]
#[allow(clippy::struct_excessive_bools)]
/// **(ID3v2 ONLY)** Flags that apply to the entire tag
pub struct TagFlags {
	/// Whether or not all frames are unsynchronised. See [`TagItemFlags::unsynchronisation`](crate::TagItemFlags::unsynchronisation)
	pub unsynchronisation: bool,
	/// Indicates if the tag is in an experimental stage
	pub experimental: bool,
	/// Indicates that the tag includes a footer
	pub footer: bool,
	/// Whether or not to include a CRC-32 in the extended header
	///
	/// This is calculated if the tag is written
	pub crc: bool,
	#[cfg(feature = "id3v2_restrictions")]
	/// Restrictions on the tag, written in the extended header
	///
	/// In addition to being setting this flag, all restrictions must be provided. See [`TagRestrictions`]
	pub restrictions: (bool, TagRestrictions),
}

#[derive(Clone)]
/// Represents a parsed tag
///
/// NOTE: Items and pictures are separated
pub struct Tag {
	tag_type: TagType,
	pictures: Vec<Picture>,
	items: Vec<TagItem>,
	#[cfg(feature = "id3v2")]
	flags: TagFlags,
}

impl IntoIterator for Tag {
	type Item = TagItem;
	type IntoIter = std::vec::IntoIter<Self::Item>;

	fn into_iter(self) -> Self::IntoIter {
		self.items.into_iter()
	}
}

impl Tag {
	/// The tag's items as a slice
	pub fn as_slice(&self) -> &[TagItem] {
		&*self.items
	}

	/// Retain tag items based on the predicate
	///
	/// See [`Vec::retain`](std::vec::Vec::retain)
	pub fn retain<F>(&mut self, f: F)
	where
		F: FnMut(&TagItem) -> bool,
	{
		self.items.retain(f)
	}

	/// Find the first TagItem matching the predicate
	///
	/// See [`Iterator::find`](std::iter::Iterator::find)
	pub fn find<P>(&mut self, predicate: P) -> Option<&TagItem>
	where
		P: for<'a> FnMut(&'a &TagItem) -> bool,
	{
		self.items.iter().find(predicate)
	}
}

impl Tag {
	/// Initialize a new tag with a certain [`TagType`]
	pub fn new(tag_type: TagType) -> Self {
		Self {
			tag_type,
			pictures: vec![],
			items: vec![],
			flags: TagFlags::default(),
		}
	}

	#[cfg(feature = "id3v2")]
	/// **(ID3v2 ONLY)** Restrict the tag's flags
	pub fn set_flags(&mut self, flags: TagFlags) {
		if TagType::Id3v2 == self.tag_type {
			self.flags = flags
		}
	}
}

impl Tag {
	/// Change the [`TagType`], remapping all items
	pub fn re_map(&mut self, tag_type: TagType) {
		self.retain(|i| i.re_map(&tag_type).is_some());
		self.tag_type = tag_type
	}
}

impl Tag {
	/// Returns the [`TagType`]
	pub fn tag_type(&self) -> &TagType {
		&self.tag_type
	}

	/// Returns the number of [`Picture`]s
	pub fn picture_count(&self) -> u32 {
		self.pictures.len() as u32
	}

	/// Returns the number of [`TagItem`]s
	pub fn item_count(&self) -> u32 {
		self.items.len() as u32
	}

	#[cfg(feature = "id3v2")]
	/// Returns the [`TagFlags`]
	pub fn flags(&self) -> &TagFlags {
		&self.flags
	}
}

impl Tag {
	/// Returns the stored [`Picture`]s as a slice
	pub fn pictures(&self) -> &[Picture] {
		&*self.pictures
	}

	/// Pushes a [`Picture`] to the tag
	pub fn push_picture(&mut self, picture: Picture) {
		self.pictures.push(picture)
	}

	/// Removes all [`Picture`]s of a [`PictureType`]
	pub fn remove_picture_type(&mut self, picture_type: PictureType) {
		self.pictures
			.iter()
			.position(|p| p.pic_type == picture_type)
			.map(|pos| self.pictures.remove(pos));
	}

	/// Removes any matching [`Picture`]
	pub fn remove_picture(&mut self, picture: &Picture) {
		self.pictures.retain(|p| p != picture)
	}
}

impl Tag {
	/// Returns the stored [`TagItem`]s as a slice
	pub fn items(&self) -> &[TagItem] {
		&*self.items
	}

	/// Returns a reference to a [`TagItem`] matching an [`ItemKey`]
	pub fn get_item_ref(&self, item_key: &ItemKey) -> Option<&TagItem> {
		self.items.iter().find(|i| &i.item_key == item_key)
	}

	/// Insert a [`TagItem`], replacing any existing one of the same type
	///
	/// NOTES:
	///
	/// * This **will** respect [`TagItemFlags::read_only`](crate::TagItemFlags::read_only)
	/// * This **will** verify an [`ItemKey`] mapping exists for the target [`TagType`]
	///
	/// # Warning
	///
	/// When dealing with ID3v2, it may be necessary to use [`insert_item_unchecked`](Tag::insert_item_unchecked).
	/// See [`id3`](crate::id3) for an explanation.
	pub fn insert_item(&mut self, item: TagItem) -> bool {
		if item.re_map(&self.tag_type).is_some() {
			self.insert_item_unchecked(item);
			return true;
		}

		false
	}

	/// Insert a [`TagItem`], replacing any existing one of the same type
	///
	/// Notes:
	///
	/// * This **will not** respect [`TagItemFlags::read_only`](crate::TagItemFlags::read_only)
	/// * This **will not** verify an [`ItemKey`] mapping exists
	/// * This **will not** allow writing item keys that are out of spec (keys are verified before writing)
	///
	/// This is only necessary if using [`ItemKey::Unknown`] or single [`ItemKey`]s that are parts of larger lists.
	pub fn insert_item_unchecked(&mut self, item: TagItem) {
		match self.items.iter_mut().find(|i| i.item_key == item.item_key) {
			None => self.items.push(item),
			Some(i) => *i = item,
		};
	}
}

impl Tag {
	/// Save the Tag to a [`File`](std::fs::File)
	///
	/// # Errors
	///
	/// * A [`FileType`](crate::FileType) couldn't be determined from the File
	/// * Attempting to write a tag to a format that does not support it. See [`FileType::supports_tag_type`](crate::FileType::supports_tag_type)
	pub fn save_to(&self, file: &mut File) -> Result<()> {
		match Probe::new().file_type(file) {
			Some(file_type) => {
				if file_type.supports_tag_type(self.tag_type()) {
					crate::logic::write_tag(self, file, file_type)
				} else {
					Err(LoftyError::UnsupportedTag)
				}
			},
			None => Err(LoftyError::UnknownFormat),
		}
	}
}

#[cfg(feature = "quick_tag_accessors")]
common_items!(TrackArtist => artist, TrackTitle => title, AlbumTitle => album_title, AlbumArtist => album_artist);

/// The tag's format
#[derive(Clone, Debug, PartialEq)]
pub enum TagType {
	#[cfg(feature = "ape")]
	/// This covers both APEv1 and APEv2 as it doesn't matter much
	Ape,
	#[cfg(feature = "id3v1")]
	/// Represents an ID3v1 tag
	Id3v1,
	#[cfg(feature = "id3v2")]
	/// This covers all ID3v2 versions since they all get upgraded to ID3v2.4
	Id3v2,
	#[cfg(feature = "mp4_atoms")]
	/// Represents MP4 atoms
	Mp4Atom,
	#[cfg(feature = "vorbis_comments")]
	/// Represents vorbis comments
	VorbisComments,
	#[cfg(feature = "riff_info_list")]
	/// Represents a RIFF INFO LIST
	RiffInfo,
	#[cfg(feature = "aiff_text_chunks")]
	/// Represents AIFF text chunks
	AiffText,
}
