use super::item::ItemKey;
use super::picture::{Picture, PictureType};
use crate::logic::id3::v2::Id3v2Version;

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
						self.get_item_ref(&ItemKey::$item_key).map(|i| i.value())
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

/// Represents a tag item (key/value)
pub struct TagItem {
	item_key: ItemKey,
	item_value: ItemValue,
}

impl TagItem {
	/// Create a new [`TagItem`]
	///
	/// NOTES:
	///
	/// * This will check for validity based on the [`TagType`].
	/// * If the [`ItemKey`] does not map to a key in the target format, `None` will be returned.
	/// * It is pointless to do this if you plan on using [`Tag::insert_item`], as it does validity checks itself.
	pub fn new_checked(
		tag_type: &TagType,
		item_key: ItemKey,
		item_value: ItemValue,
	) -> Option<Self> {
		item_key.map_key(tag_type).is_some().then(|| Self {
			item_key,
			item_value,
		})
	}

	/// Create a new [`TagItem`]
	pub fn new(item_key: ItemKey, item_value: ItemValue) -> Self {
		Self {
			item_key,
			item_value,
		}
	}

	/// Returns a reference to the [`ItemKey`]
	pub fn key(&self) -> &ItemKey {
		&self.item_key
	}

	/// Returns a reference to the [`ItemValue`]
	pub fn value(&self) -> &ItemValue {
		&self.item_value
	}

	pub(crate) fn re_map(self, tag_type: &TagType) -> Option<Self> {
		self.item_key.map_key(tag_type).is_some().then(|| self)
	}
}

/// Represents a tag item's value
///
/// NOTE: The [Locator][ItemValue::Locator] and [Binary][ItemValue::Binary] variants are only applicable to APE tags.
/// Attempting to write either to another file/tag type will **not** error, they will just be ignored.
pub enum ItemValue {
	/// Any UTF-8 encoded text
	Text(String),
	/// **(APE ONLY)** Any UTF-8 encoded locator of external information
	Locator(String),
	/// **(APE ONLY)** Binary information, most likely a picture
	Binary(Vec<u8>),
}

/// Represents a parsed tag
///
/// NOTE: Items and pictures are separated
pub struct Tag {
	tag_type: TagType,
	pictures: Vec<Picture>,
	items: Vec<TagItem>,
}

impl IntoIterator for Tag {
	type Item = TagItem;
	type IntoIter = std::vec::IntoIter<Self::Item>;

	fn into_iter(self) -> Self::IntoIter {
		self.items.into_iter()
	}
}

impl Tag {
	/// An iterator over the tag's items
	pub fn iter(&self) -> std::slice::Iter<TagItem> {
		self.items.iter()
	}

	/// Retain tag items based on the predicate
	///
	/// See [`Vec::retain`](std::vec::Vec::retain)
	pub fn retain<F>(&mut self, mut f: F)
	where
		F: FnMut(&TagItem) -> bool,
	{
		self.items.retain(f)
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

	/// Insert a [`TagItem`], replacing any existing one of the same type
	///
	/// # Returns
	///
	/// This returns a bool if the item was successfully inserted/replaced.
	///
	/// `false` is only returned if the [`TagItem`]'s key couldn't be remapped to the target [`TagType`]
	pub fn insert_item(&mut self, item: TagItem) -> bool {
		if let Some(item) = item.re_map(&self.tag_type) {
			match self.items.iter_mut().find(|i| i.item_key == item.item_key) {
				None => self.items.push(item),
				Some(i) => *i = item,
			};

			return true;
		}

		false
	}
}

#[cfg(feature = "quick_tag_accessors")]
common_items!(Artist => artist, Title => title, AlbumTitle => album_title, AlbumArtist => album_artist);

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
	/// This covers all ID3v2 versions.
	///
	/// Due to frame differences between versions, it is necessary it be specified. See [`Id3v2Version`](crate::id3::Id3v2Version) for variants.
	Id3v2(Id3v2Version),
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
