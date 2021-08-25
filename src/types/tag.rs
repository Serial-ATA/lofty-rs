use super::item::ItemKey;
use super::picture::{Picture, PictureType};
use crate::logic::id3::v2::restrictions::TagRestrictions;
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

#[derive(Clone, Debug)]
#[allow(clippy::struct_excessive_bools)]
/// **(ID3v2/APEv2 ONLY)** Various flags to describe the content of an item
///
/// It is not an error to attempt to write flags to a format that doesn't support them.
/// They will just be ignored.
pub struct TagItemFlags {
	/// **(ID3v2 ONLY)** Preserve frame on tag edit
	pub tag_alter_preservation: bool,
	/// **(ID3v2 ONLY)** Preserve frame on file edit
	pub file_alter_preservation: bool,
	/// **(ID3v2/APEv2 ONLY)** Item cannot be written to
	pub read_only: bool,
	/// **(ID3v2 ONLY)** Frame belongs in a group
	///
	/// In addition to setting this flag, a group identifier byte must be added.
	/// All frames with the same group identifier byte belong to the same group.
	pub grouping_identity: (bool, u8),
	/// **(ID3v2 ONLY)** Frame is zlib compressed
	///
	/// It is **required** `data_length_indicator` be set if this is set.
	pub compression: bool,
	/// **(ID3v2 ONLY)** Frame is encrypted
	///
	/// NOTE: Since the encryption method is unknown, lofty cannot do anything with these frames
	///
	/// In addition to setting this flag, an encryption method symbol must be added.
	/// The method symbol **must** be > 0x80.
	pub encryption: (bool, u8),
	/// **(ID3v2 ONLY)** Frame is unsynchronised
	///
	/// In short, this makes all "0xFF 0x00" combinations into "0xFF 0x00 0x00" to avoid confusion
	/// with the MPEG frame header, which is often identified by its "frame sync" (11 set bits).
	/// It is preferred an ID3v2 tag is either *completely* unsynchronised or not unsynchronised at all.
	pub unsynchronisation: bool,
	/// **(ID3v2 ONLY)** Frame has a data length indicator
	///
	/// The data length indicator is the size of the frame if the flags were all zeroed out.
	/// This is usually used in combination with `compression` and `encryption` (depending on encryption method).
	///
	/// In addition to setting this flag, the final size must be added.
	pub data_length_indicator: (bool, u32),
}

impl Default for TagItemFlags {
	fn default() -> Self {
		Self {
			tag_alter_preservation: false,
			file_alter_preservation: false,
			read_only: false,
			grouping_identity: (false, 0),
			compression: false,
			encryption: (false, 0),
			unsynchronisation: false,
			data_length_indicator: (false, 0),
		}
	}
}

#[derive(Clone, Debug)]
/// Represents a tag item (key/value)
pub struct TagItem {
	item_key: ItemKey,
	item_value: ItemValue,
	flags: TagItemFlags,
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
			flags: TagItemFlags::default(),
		})
	}

	/// Create a new [`TagItem`]
	pub fn new(item_key: ItemKey, item_value: ItemValue) -> Self {
		Self {
			item_key,
			item_value,
			flags: TagItemFlags::default(),
		}
	}

	/// Set the item's flags
	pub fn set_flags(&mut self, flags: TagItemFlags) {
		self.flags = flags
	}

	/// Returns a reference to the [`ItemKey`]
	pub fn key(&self) -> &ItemKey {
		&self.item_key
	}

	/// Returns a reference to the [`ItemValue`]
	pub fn value(&self) -> &ItemValue {
		&self.item_value
	}

	/// Returns a reference to the [`TagItemFlags`]
	pub fn flags(&self) -> &TagItemFlags {
		&self.flags
	}

	pub(crate) fn re_map(&self, tag_type: &TagType) -> Option<()> {
		self.item_key.map_key(tag_type).is_some().then(|| ())
	}
}

#[derive(Clone, Debug)]
/// Represents a tag item's value
///
/// NOTES:
///
/// * The [Locator][ItemValue::Locator] variant is only applicable to APE and ID3v2 tags.
/// * The [Binary][ItemValue::Binary] variant is only applicable to APE tags.
/// * Attempting to write either to another file/tag type will **not** error, they will just be ignored.
pub enum ItemValue {
	/// Any UTF-8 encoded text
	Text(String),
	/// **(APE/ID3v2 ONLY)** Any UTF-8 encoded locator of external information
	Locator(String),
	/// **(APE/ID3v2 ONLY)** Binary information
	///
	/// In the case of ID3v2, this is the type of a [`Id3v2Frame::EncapsulatedObject`](crate::id3::Id3v2Frame::EncapsulatedObject) **and** any unknown frame.
	///
	/// For APEv2, no uses of this item type are documented, there's no telling what it could be.
	Binary(Vec<u8>),
	/// **(ID3v2 ONLY)** The content of a synchronized text frame, see [`SynchronizedText`](crate::id3::SynchronizedText)
	SynchronizedText(Vec<(u32, String)>),
}

#[derive(Default, Copy, Clone)]
#[allow(clippy::struct_excessive_bools)]
/// **(ID3v2 ONLY)** Flags that apply to the entire tag
pub struct TagFlags {
	/// Whether or not all frames are unsynchronised. See [`TagItemFlags::unsynchronization`]
	pub unsynchronisation: bool,
	/// Whether or not the header is followed by an extended header
	pub extended_header: bool,
	/// Indicates if the tag is in an experimental stage
	pub experimental: bool,
	/// Indicates that the tag includes a footer
	pub footer: bool,
	/// Whether or not to include a CRC-32 in the extended header
	///
	/// NOTE: This **requires** `extended_header` to be set. Otherwise, it will be ignored.
	///
	/// This is calculated if the tag is written
	pub crc: bool,
	/// Restrictions on the tag
	///
	/// NOTE: This **requires** `extended_header` to be set. Otherwise, it will be ignored.
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
	/// * This **will** respect [`TagItemFlags::read_only`]
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
	/// * This **will not** respect [`TagItemFlags::read_only`]
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
