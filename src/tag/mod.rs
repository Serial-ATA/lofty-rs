pub(crate) mod item;
pub(crate) mod utils;

use crate::error::{LoftyError, Result};
use crate::file::FileType;
use crate::macros::err;
use crate::picture::{Picture, PictureType};
use crate::probe::Probe;
use crate::traits::{Accessor, TagExt};
use item::{ItemKey, ItemValue, TagItem};

use std::borrow::Cow;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

macro_rules! impl_accessor {
	($($item_key:ident => $name:tt),+) => {
		paste::paste! {
			$(
				fn $name(&self) -> Option<Cow<'_, str>> {
					if let Some(ItemValue::Text(txt)) = self.get_item_ref(&ItemKey::$item_key).map(TagItem::value) {
						return Some(Cow::Borrowed(txt))
					}

					None
				}

				fn [<set_ $name>](&mut self, value: String) {
					if value.is_empty() {
						self.[<remove_ $name>]();
						return;
					}

					self.insert_item(TagItem::new(ItemKey::$item_key, ItemValue::Text(value)));
				}

				fn [<remove_ $name>](&mut self) {
					self.retain_items(|i| i.item_key != ItemKey::$item_key)
				}
			)+
		}
	}
}

/// Represents a parsed tag
///
/// This is a tag that is loosely bound to a specific [`TagType`].
/// It is used for conversions and as the return type for [`read_from`](crate::read_from).
///
/// Compared to other formats, this gives a much higher-level view of the
/// tag items. Rather than storing items according to their format-specific
/// keys, [`ItemKey`]s are used.
///
/// You can easily remap this to another [`TagType`] with [`Tag::re_map`].
///
/// Any conversion will, of course, be lossy to a varying degree.
///
/// ## Usage
///
/// Accessing common items
///
/// ```rust
/// use lofty::{Accessor, Tag, TagType};
///
/// let tag = Tag::new(TagType::ID3v2);
///
/// // There are multiple quick getter methods for common items
///
/// let title = tag.title();
/// let artist = tag.artist();
/// let album = tag.album();
/// let genre = tag.genre();
/// ```
///
/// Getting an item of a known type
///
/// ```rust
/// use lofty::{ItemKey, Tag, TagType};
///
/// let tag = Tag::new(TagType::ID3v2);
///
/// // If the type of an item is known, there are getter methods
/// // to prevent having to match against the value
///
/// tag.get_string(&ItemKey::TrackTitle);
/// tag.get_binary(&ItemKey::TrackTitle, false);
/// ```
///
/// Converting between formats
///
/// ```rust
/// use lofty::id3::v2::ID3v2Tag;
/// use lofty::{Tag, TagType};
///
/// // Converting between formats is as simple as an `into` call.
/// // However, such conversions can potentially be *very* lossy.
///
/// let tag = Tag::new(TagType::ID3v2);
/// let id3v2_tag: ID3v2Tag = tag.into();
/// ```
#[derive(Clone)]
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

impl Accessor for Tag {
	impl_accessor!(
		TrackArtist => artist,
		TrackTitle  => title,
		AlbumTitle  => album,
		Genre       => genre,
		Comment     => comment
	);

	fn track(&self) -> Option<u32> {
		if let Some(i) = self.get_string(&ItemKey::TrackNumber) {
			return i.parse::<u32>().ok();
		}

		None
	}

	fn set_track(&mut self, value: u32) {
		self.insert_text(ItemKey::TrackNumber, value.to_string());
	}

	fn remove_track(&mut self) {
		self.remove_key(&ItemKey::TrackNumber);
	}

	fn track_total(&self) -> Option<u32> {
		if let Some(i) = self.get_string(&ItemKey::TrackTotal) {
			return i.parse::<u32>().ok();
		}

		None
	}

	fn set_track_total(&mut self, value: u32) {
		self.insert_text(ItemKey::TrackTotal, value.to_string());
	}

	fn remove_track_total(&mut self) {
		self.remove_key(&ItemKey::TrackTotal);
	}

	fn disk(&self) -> Option<u32> {
		if let Some(i) = self.get_string(&ItemKey::DiscNumber) {
			return i.parse::<u32>().ok();
		}

		None
	}

	fn set_disk(&mut self, value: u32) {
		self.insert_text(ItemKey::DiscNumber, value.to_string());
	}

	fn remove_disk(&mut self) {
		self.remove_key(&ItemKey::DiscNumber);
	}

	fn disk_total(&self) -> Option<u32> {
		if let Some(i) = self.get_string(&ItemKey::DiscTotal) {
			return i.parse::<u32>().ok();
		}

		None
	}

	fn set_disk_total(&mut self, value: u32) {
		self.insert_text(ItemKey::DiscTotal, value.to_string());
	}

	fn remove_disk_total(&mut self) {
		self.remove_key(&ItemKey::DiscTotal);
	}

	fn year(&self) -> Option<u32> {
		if let Some(item) = self
			.get_string(&ItemKey::Year)
			.map_or_else(|| self.get_string(&ItemKey::RecordingDate), Some)
		{
			return item.chars().take(4).collect::<String>().parse::<u32>().ok();
		}

		None
	}

	fn set_year(&mut self, value: u32) {
		if let Some(item) = self.get_string(&ItemKey::RecordingDate) {
			if item.len() >= 4 {
				let (_, remaining) = item.split_at(4);
				self.insert_text(ItemKey::RecordingDate, format!("{value}{remaining}"));
			}
		}
	}

	fn remove_year(&mut self) {
		self.remove_key(&ItemKey::Year);
		self.remove_key(&ItemKey::RecordingDate);
	}
}

impl Tag {
	/// Initialize a new tag with a certain [`TagType`]
	pub fn new(tag_type: TagType) -> Self {
		Self {
			tag_type,
			pictures: Vec::new(),
			items: Vec::new(),
		}
	}

	/// Change the [`TagType`], remapping all items
	pub fn re_map(&mut self, tag_type: TagType) {
		self.retain_items(|i| i.re_map(tag_type));
		self.tag_type = tag_type
	}

	/// Returns the [`TagType`]
	pub fn tag_type(&self) -> TagType {
		self.tag_type
	}

	/// Returns the number of [`TagItem`]s
	pub fn item_count(&self) -> u32 {
		self.items.len() as u32
	}

	/// Returns the number of [`Picture`]s
	pub fn picture_count(&self) -> u32 {
		self.pictures.len() as u32
	}

	/// Returns the stored [`TagItem`]s as a slice
	pub fn items(&self) -> &[TagItem] {
		&self.items
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
				ItemValue::Text(text) | ItemValue::Locator(text) if convert => {
					return Some(text.as_bytes())
				},
				ItemValue::Binary(binary) => return Some(binary),
				_ => {},
			}
		}

		None
	}

	/// Insert a [`TagItem`], replacing any existing one of the same [`ItemKey`]
	///
	/// NOTE: This **will** verify an [`ItemKey`] mapping exists for the target [`TagType`]
	///
	/// This will return `true` if the item was inserted.
	pub fn insert_item(&mut self, item: TagItem) -> bool {
		if item.re_map(self.tag_type) {
			self.insert_item_unchecked(item);
			return true;
		}

		false
	}

	/// Insert a [`TagItem`], replacing any existing one of the same [`ItemKey`]
	///
	/// Notes:
	///
	/// * This **will not** verify an [`ItemKey`] mapping exists
	/// * This **will not** allow writing item keys that are out of spec (keys are verified before writing)
	///
	/// This is only necessary if dealing with [`ItemKey::Unknown`].
	pub fn insert_item_unchecked(&mut self, item: TagItem) {
		self.retain_items(|i| i.item_key != item.item_key);
		self.items.push(item);
	}

	/// Append a [`TagItem`] to the tag
	///
	/// This will not remove any items of the same [`ItemKey`], unlike [`Tag::insert_item`]
	///
	/// NOTE: This **will** verify an [`ItemKey`] mapping exists for the target [`TagType`]
	///
	/// Multiple items of the same [`ItemKey`] are not valid in all formats, in which case
	/// the first available item will be used.
	///
	/// This will return `true` if the item was pushed.
	pub fn push_item(&mut self, item: TagItem) -> bool {
		if item.re_map(self.tag_type) {
			self.items.push(item);
			return true;
		}

		false
	}

	/// Append a [`TagItem`] to the tag
	///
	/// Notes: See [`Tag::insert_item_unchecked`]
	pub fn push_item_unchecked(&mut self, item: TagItem) {
		self.items.push(item);
	}

	/// An alias for [`Tag::insert_item`] that doesn't require the user to create a [`TagItem`]
	///
	/// NOTE: This will replace any existing item with `item_key`. See [`Tag::insert_item`]
	pub fn insert_text(&mut self, item_key: ItemKey, text: String) -> bool {
		self.insert_item(TagItem::new(item_key, ItemValue::Text(text)))
	}

	/// Removes all items with the specified [`ItemKey`], and returns them
	pub fn take(&mut self, key: &ItemKey) -> impl Iterator<Item = TagItem> + '_ {
		// TODO: drain_filter
		let mut split_idx = 0_usize;

		for read_idx in 0..self.items.len() {
			if self.items[read_idx].key() == key {
				self.items.swap(split_idx, read_idx);
				split_idx += 1;
			}
		}

		self.items.drain(..split_idx)
	}

	/// Removes all items with the specified [`ItemKey`], and filters them through [`ItemValue::into_string`]
	pub fn take_strings(&mut self, key: &ItemKey) -> impl Iterator<Item = String> + '_ {
		self.take(key).filter_map(|i| i.item_value.into_string())
	}

	/// Returns references to all [`TagItem`]s with the specified key
	pub fn get_items<'a>(&'a self, key: &'a ItemKey) -> impl Iterator<Item = &'a TagItem> {
		self.items.iter().filter(move |i| i.key() == key)
	}

	/// Returns references to all texts of [`TagItem`]s with the specified key, and [`ItemValue::Text`]
	pub fn get_strings<'a>(&'a self, key: &'a ItemKey) -> impl Iterator<Item = &'a str> {
		self.items.iter().filter_map(move |i| {
			if i.key() == key {
				i.value().text()
			} else {
				None
			}
		})
	}

	/// Returns references to all locators of [`TagItem`]s with the specified key, and [`ItemValue::Locator`]
	pub fn get_locators<'a>(&'a self, key: &'a ItemKey) -> impl Iterator<Item = &'a str> {
		self.items.iter().filter_map(move |i| {
			if i.key() == key {
				i.value().locator()
			} else {
				None
			}
		})
	}

	/// Returns references to all bytes of [`TagItem`]s with the specified key, and [`ItemValue::Binary`]
	pub fn get_bytes<'a>(&'a self, key: &'a ItemKey) -> impl Iterator<Item = &'a [u8]> {
		self.items.iter().filter_map(move |i| {
			if i.key() == key {
				i.value().binary()
			} else {
				None
			}
		})
	}

	/// Remove an item by its key
	///
	/// This will remove all items with this key.
	pub fn remove_key(&mut self, key: &ItemKey) {
		self.items.retain(|i| i.key() != key)
	}

	/// Retain tag items based on the predicate
	///
	/// See [`Vec::retain`](std::vec::Vec::retain)
	pub fn retain_items<F>(&mut self, f: F)
	where
		F: FnMut(&TagItem) -> bool,
	{
		self.items.retain(f)
	}

	/// Returns the stored [`Picture`]s as a slice
	pub fn pictures(&self) -> &[Picture] {
		&self.pictures
	}

	/// Returns the first occurrence of the [`PictureType`]
	pub fn get_picture_type(&self, picture_type: PictureType) -> Option<&Picture> {
		self.pictures
			.iter()
			.find(|picture| picture.pic_type() == picture_type)
	}

	/// Pushes a [`Picture`] to the tag
	pub fn push_picture(&mut self, picture: Picture) {
		self.pictures.push(picture)
	}

	/// Removes all [`Picture`]s of a [`PictureType`]
	pub fn remove_picture_type(&mut self, picture_type: PictureType) {
		self.pictures.retain(|p| p.pic_type != picture_type)
	}

	/// Replaces the picture at the given `index`
	///
	/// NOTE: If `index` is out of bounds, the `picture` will be appended
	/// to the list.
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::{Picture, Tag, TagType};
	/// # use lofty::{PictureType, MimeType};
	///
	/// # let front_cover = Picture::new_unchecked(PictureType::CoverFront, MimeType::Png, None, Vec::new());
	/// # let back_cover = Picture::new_unchecked(PictureType::CoverBack, MimeType::Png, None, Vec::new());
	/// # let another_picture = Picture::new_unchecked(PictureType::Band, MimeType::Png, None, Vec::new());
	/// let mut tag = Tag::new(TagType::ID3v2);
	///
	/// // Add a front cover
	/// tag.push_picture(front_cover);
	///
	/// assert_eq!(tag.pictures().len(), 1);
	/// assert_eq!(tag.pictures()[0].pic_type(), PictureType::CoverFront);
	///
	/// // Replace the front cover with a back cover
	/// tag.set_picture(0, back_cover);
	///
	/// assert_eq!(tag.pictures().len(), 1);
	/// assert_eq!(tag.pictures()[0].pic_type(), PictureType::CoverBack);
	///
	/// // Use an out of bounds index
	/// tag.set_picture(100, another_picture);
	///
	/// assert_eq!(tag.pictures().len(), 2);
	/// ```
	pub fn set_picture(&mut self, index: usize, picture: Picture) {
		if index >= self.pictures.len() {
			self.push_picture(picture);
		} else {
			self.pictures[index] = picture;
		}
	}

	/// Removes and returns the picture at the given `index`
	///
	/// # Panics
	///
	/// Panics if `index` is out of bounds.
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::{Picture, Tag, TagType};
	/// # use lofty::{PictureType, MimeType};
	///
	/// # let picture = Picture::new_unchecked(PictureType::CoverFront, MimeType::Png, None, Vec::new());
	/// let mut tag = Tag::new(TagType::ID3v2);
	/// tag.push_picture(picture);
	///
	/// assert_eq!(tag.pictures().len(), 1);
	///
	/// tag.remove_picture(0);
	///
	/// assert_eq!(tag.pictures().len(), 0);
	/// ```
	pub fn remove_picture(&mut self, index: usize) -> Picture {
		self.pictures.remove(index)
	}
}

impl TagExt for Tag {
	type Err = LoftyError;
	type RefKey<'a> = &'a ItemKey;

	fn contains<'a>(&'a self, key: Self::RefKey<'a>) -> bool {
		self.items.iter().any(|item| item.key() == key)
	}

	fn is_empty(&self) -> bool {
		self.items.is_empty() && self.pictures.is_empty()
	}

	/// Save the `Tag` to a path
	///
	/// # Errors
	///
	/// * Path doesn't exist
	/// * Path is not writable
	/// * See [`Tag::save_to`]
	fn save_to_path<P: AsRef<Path>>(&self, path: P) -> std::result::Result<(), Self::Err> {
		self.save_to(&mut OpenOptions::new().read(true).write(true).open(path)?)
	}

	/// Save the `Tag` to a [`File`](std::fs::File)
	///
	/// # Errors
	///
	/// * A [`FileType`](crate::FileType) couldn't be determined from the File
	/// * Attempting to write a tag to a format that does not support it. See [`FileType::supports_tag_type`](crate::FileType::supports_tag_type)
	fn save_to(&self, file: &mut File) -> std::result::Result<(), Self::Err> {
		let probe = Probe::new(file).guess_file_type()?;

		match probe.file_type() {
			Some(file_type) => {
				if file_type.supports_tag_type(self.tag_type()) {
					utils::write_tag(self, probe.into_inner(), file_type)
				} else {
					err!(UnsupportedTag);
				}
			},
			None => err!(UnknownFormat),
		}
	}

	fn dump_to<W: Write>(&self, writer: &mut W) -> Result<()> {
		utils::dump_tag(self, writer)
	}

	/// Remove a tag from a [`Path`]
	///
	/// # Errors
	///
	/// See [`TagType::remove_from`]
	fn remove_from_path<P: AsRef<Path>>(&self, path: P) -> std::result::Result<(), Self::Err> {
		self.tag_type.remove_from_path(path)
	}

	/// Remove a tag from a [`File`]
	///
	/// # Errors
	///
	/// See [`TagType::remove_from`]
	fn remove_from(&self, file: &mut File) -> std::result::Result<(), Self::Err> {
		self.tag_type.remove_from(file)
	}

	fn clear(&mut self) {
		self.items.clear();
		self.pictures.clear();
	}
}

/// The tag's format
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TagType {
	/// This covers both APEv1 and APEv2 as it doesn't matter much
	APE,
	/// Represents an ID3v1 tag
	ID3v1,
	/// This covers all ID3v2 versions since they all get upgraded to ID3v2.4
	ID3v2,
	/// Represents an MP4 ilst atom
	MP4ilst,
	/// Represents vorbis comments
	VorbisComments,
	/// Represents a RIFF INFO LIST
	RIFFInfo,
	/// Represents AIFF text chunks
	AIFFText,
}

impl TagType {
	/// Remove a tag from a [`Path`]
	///
	/// # Errors
	///
	/// See [`TagType::remove_from`]
	pub fn remove_from_path(&self, path: impl AsRef<Path>) -> Result<()> {
		let mut file = OpenOptions::new().read(true).write(true).open(path)?;
		self.remove_from(&mut file)
	}

	#[allow(clippy::shadow_unrelated)]
	/// Remove a tag from a [`File`]
	///
	/// # Errors
	///
	/// * It is unable to guess the file format
	/// * The format doesn't support the tag
	/// * It is unable to write to the file
	pub fn remove_from(&self, file: &mut File) -> Result<()> {
		let probe = Probe::new(file).guess_file_type()?;
		let file_type = match probe.file_type() {
			Some(f_ty) => f_ty,
			None => err!(UnknownFormat),
		};

		let special_exceptions =
			(file_type == FileType::APE || file_type == FileType::FLAC) && *self == TagType::ID3v2;

		if !special_exceptions && !file_type.supports_tag_type(*self) {
			err!(UnsupportedTag);
		}

		let file = probe.into_inner();
		utils::write_tag(&Tag::new(*self), file, file_type)
	}
}

#[cfg(test)]
mod tests {
	use crate::tag::utils::test_utils::read_path;
	use crate::{Accessor, Picture, PictureType, Tag, TagExt, TagType};
	use std::io::{Seek, Write};
	use std::process::Command;

	#[test]
	fn issue_37() {
		let file_contents = read_path("tests/files/assets/issue_37.ogg");
		let mut temp_file = tempfile::NamedTempFile::new().unwrap();
		temp_file.write_all(&file_contents).unwrap();
		temp_file.rewind().unwrap();

		let mut tag = Tag::new(TagType::VorbisComments);

		let mut picture =
			Picture::from_reader(&mut &*read_path("tests/files/assets/issue_37.jpg")).unwrap();
		picture.set_pic_type(PictureType::CoverFront);

		tag.push_picture(picture);
		tag.save_to(temp_file.as_file_mut()).unwrap();

		let cmd_output = Command::new("ffprobe")
			.arg(temp_file.path().to_str().unwrap())
			.output()
			.unwrap();

		assert!(cmd_output.status.success());

		let stderr = String::from_utf8(cmd_output.stderr).unwrap();

		assert!(!stderr.contains("CRC mismatch!"));
		assert!(
			!stderr.contains("Header processing failed: Invalid data found when processing input")
		);
	}

	#[test]
	fn insert_empty() {
		let mut tag = Tag::new(TagType::ID3v2);
		tag.set_title(String::from("Foo title"));

		assert_eq!(tag.title().as_deref(), Some("Foo title"));

		tag.set_title(String::new());
		assert_eq!(tag.title(), None);
	}
}
