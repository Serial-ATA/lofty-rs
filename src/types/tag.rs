use super::item::{ItemKey, ItemValue, TagItem};
use super::picture::{Picture, PictureType};
use crate::error::{LoftyError, Result};
use crate::probe::Probe;

use std::fs::{File, OpenOptions};
use std::path::Path;

use paste::paste;

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

#[derive(Clone)]
/// Represents a parsed tag
///
/// NOTE: Items and pictures are separated
pub struct Tag {
	tag_type: TagType,
	pub(crate) pictures: Vec<Picture>,
	pub(crate) items: Vec<TagItem>,
}

impl IntoIterator for Tag {
	type Item = TagItem;
	type IntoIter = std::vec::IntoIter<Self::Item>;

	fn into_iter(self) -> Self::IntoIter {
		self.items.into_iter()
	}
}

impl Tag {
	/// Initialize a new tag with a certain [`TagType`]
	pub fn new(tag_type: TagType) -> Self {
		Self {
			tag_type,
			pictures: vec![],
			items: vec![],
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

	/// Get a string value from an [`ItemKey`]
	pub fn get_string(&self, item_key: &ItemKey) -> Option<&str> {
		if let Some(ItemValue::Text(ret)) = self.get_item_ref(item_key).map(TagItem::value) {
			return Some(ret);
		}

		None
	}

	/// Gets a byte slice from an [`ItemKey`]
	///
	/// Use `convert` to convert [`ItemValue::Text`] and [`ItemValue::Locator`] to byte slices
	pub fn get_binary(&self, item_key: &ItemKey, convert: bool) -> Option<&[u8]> {
		if let Some(item) = self.get_item_ref(item_key) {
			match item.value() {
				ItemValue::Text(text) if convert => return Some(text.as_bytes()),
				ItemValue::Locator(locator) => return Some(locator.as_bytes()),
				ItemValue::Binary(binary) => return Some(binary),
				_ => {}
			}
		}

		None
	}

	/// Insert a [`TagItem`], replacing any existing one of the same type
	///
	/// NOTE: This **will** verify an [`ItemKey`] mapping exists for the target [`TagType`]
	///
	/// # Warning
	///
	/// When dealing with ID3v2, it may be necessary to use [`insert_item_unchecked`](Tag::insert_item_unchecked).
	/// See [`id3`](crate::id3::v2) for an explanation.
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
	/// * This **will not** verify an [`ItemKey`] mapping exists
	/// * This **will not** allow writing item keys that are out of spec (keys are verified before writing)
	///
	/// This is only necessary if dealing with [`ItemKey::Unknown`].
	pub fn insert_item_unchecked(&mut self, item: TagItem) {
		match self.items.iter_mut().find(|i| i.item_key == item.item_key) {
			None => self.items.push(item),
			Some(i) => *i = item,
		};
	}

	/// An alias for [`Tag::insert_item`] that doesn't require the user to create a [`TagItem`]
	pub fn insert_text(&mut self, item_key: ItemKey, text: String) -> bool {
		self.insert_item(TagItem::new(item_key, ItemValue::Text(text)))
	}
}

impl Tag {
	/// Save the `Tag` to a path
	///
	/// # Errors
	///
	/// * Path doesn't exist
	/// * Path is not writable
	/// * See [`Tag::save_to`]
	pub fn save_to_path(&self, path: impl AsRef<Path>) -> Result<()> {
		self.save_to(&mut OpenOptions::new().read(true).write(true).open(path)?)
	}

	/// Save the `Tag` to a [`File`](std::fs::File)
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
			}
			None => Err(LoftyError::UnknownFormat),
		}
	}

	/// Same as [`TagType::remove_from_path`]
	pub fn remove_from_path(&self, path: impl AsRef<Path>) -> bool {
		self.tag_type.remove_from_path(path)
	}

	/// Same as [`TagType::remove_from`]
	pub fn remove_from(&self, file: &mut File) -> bool {
		self.tag_type.remove_from(file)
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

impl TagType {
	/// Remove a tag from a [`Path`]
	///
	/// See [`TagType::remove_from`]
	pub fn remove_from_path(&self, path: impl AsRef<Path>) -> bool {
		if let Ok(mut file) = OpenOptions::new().read(true).write(true).open(path) {
			return self.remove_from(&mut file);
		}

		false
	}

	/// Remove a tag from a [`File`]
	///
	/// This will return `false` if:
	///
	/// * It is unable to guess the file format
	/// * The format doesn't support the `TagType`
	/// * It is unable to write to the file
	pub fn remove_from(&self, file: &mut File) -> bool {
		if let Some(file_type) = Probe::new().file_type(file) {
			if file_type.supports_tag_type(self) {
				return crate::logic::write_tag(&Tag::new(self.clone()), file, file_type).is_ok();
			}
		}

		false
	}
}
