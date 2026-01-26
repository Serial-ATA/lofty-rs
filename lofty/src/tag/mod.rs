//! Utilities for generic tag handling

mod accessor;
pub(crate) mod companion_tag;
pub(crate) mod item;
pub mod items;
mod split_merge_tag;
mod tag_ext;
mod tag_type;
pub(crate) mod utils;

use crate::config::{ParsingMode, WriteOptions};
use crate::error::{LoftyError, Result};
use crate::macros::err;
use crate::picture::{Picture, PictureType};
use crate::probe::Probe;
use crate::tag::items::Timestamp;
use crate::tag::items::popularimeter::Popularimeter;
use crate::util::io::{FileLike, Length, Truncate};

use std::borrow::Cow;
use std::io::Write;
use std::path::Path;

// Exports
pub use accessor::Accessor;
pub use item::{ItemKey, ItemValue, TagItem};
pub use split_merge_tag::{MergeTag, SplitTag};
pub use tag_ext::TagExt;
pub use tag_type::{TagSupport, TagType};

macro_rules! impl_accessor {
	($($item_key:ident => $name:tt),+) => {
		paste::paste! {
			$(
				fn $name(&self) -> Option<Cow<'_, str>> {
					if let Some(ItemValue::Text(txt)) = self.get(ItemKey::$item_key).map(TagItem::value) {
						return Some(Cow::Borrowed(txt))
					}

					None
				}

				fn [<set_ $name>](&mut self, value: String) {
					self.insert(TagItem::new(ItemKey::$item_key, ItemValue::Text(value)));
				}

				fn [<remove_ $name>](&mut self) {
					self.retain(|i| i.item_key != ItemKey::$item_key)
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
/// use lofty::tag::{Accessor, Tag, TagType};
///
/// let tag = Tag::new(TagType::Id3v2);
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
/// use lofty::tag::{ItemKey, Tag, TagType};
///
/// let tag = Tag::new(TagType::Id3v2);
///
/// // If the type of an item is known, there are getter methods
/// // to prevent having to match against the value
///
/// tag.get_string(ItemKey::TrackTitle);
/// tag.get_binary(ItemKey::TrackTitle, false);
/// ```
///
/// Converting between formats
///
/// ```rust
/// use lofty::id3::v2::Id3v2Tag;
/// use lofty::tag::{Tag, TagType};
///
/// // Converting between formats is as simple as an `into` call.
/// // However, such conversions can potentially be *very* lossy.
///
/// let tag = Tag::new(TagType::Id3v2);
/// let id3v2_tag: Id3v2Tag = tag.into();
/// ```
#[derive(Clone)]
pub struct Tag {
	tag_type: TagType,
	pub(crate) pictures: Vec<Picture>,
	pub(crate) items: Vec<TagItem>,
	pub(crate) companion_tag: Option<companion_tag::CompanionTag>,
}

#[must_use]
pub(crate) fn try_parse_timestamp(input: &str) -> Option<Timestamp> {
	let Ok(timestamp) = Timestamp::parse(&mut input.as_bytes(), ParsingMode::Relaxed) else {
		log::warn!("Timestamp exists in file, but cannot be parsed.");
		return None;
	};

	timestamp
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
		self.get_u32_from_string(ItemKey::TrackNumber)
	}

	fn set_track(&mut self, value: u32) {
		self.insert_text(ItemKey::TrackNumber, value.to_string());
	}

	fn remove_track(&mut self) {
		self.remove_key(ItemKey::TrackNumber);
	}

	fn track_total(&self) -> Option<u32> {
		self.get_u32_from_string(ItemKey::TrackTotal)
	}

	fn set_track_total(&mut self, value: u32) {
		self.insert_text(ItemKey::TrackTotal, value.to_string());
	}

	fn remove_track_total(&mut self) {
		self.remove_key(ItemKey::TrackTotal);
	}

	fn disk(&self) -> Option<u32> {
		self.get_u32_from_string(ItemKey::DiscNumber)
	}

	fn set_disk(&mut self, value: u32) {
		self.insert_text(ItemKey::DiscNumber, value.to_string());
	}

	fn remove_disk(&mut self) {
		self.remove_key(ItemKey::DiscNumber);
	}

	fn disk_total(&self) -> Option<u32> {
		self.get_u32_from_string(ItemKey::DiscTotal)
	}

	fn set_disk_total(&mut self, value: u32) {
		self.insert_text(ItemKey::DiscTotal, value.to_string());
	}

	fn remove_disk_total(&mut self) {
		self.remove_key(ItemKey::DiscTotal);
	}

	fn date(&self) -> Option<Timestamp> {
		self.get_string(ItemKey::RecordingDate)
			.or_else(|| self.get_string(ItemKey::Year))
			.and_then(|d| {
				Timestamp::parse(&mut d.as_bytes(), ParsingMode::Relaxed)
					.ok()
					.flatten()
			})
	}

	fn set_date(&mut self, value: Timestamp) {
		self.remove_key(ItemKey::Year);
		self.insert_text(ItemKey::RecordingDate, value.to_string());
	}

	fn remove_date(&mut self) {
		self.remove_key(ItemKey::Year);
		self.remove_key(ItemKey::RecordingDate);
	}
}

impl Tag {
	/// Initialize a new tag with a certain [`TagType`]
	#[must_use]
	pub const fn new(tag_type: TagType) -> Self {
		Self {
			tag_type,
			pictures: Vec::new(),
			items: Vec::new(),
			companion_tag: None,
		}
	}

	/// Change the [`TagType`], remapping all items
	///
	/// NOTE: If any format-specific items are present, they will be removed.
	///       See [`GlobalOptions::preserve_format_specific_items`].
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::tag::{Accessor, Tag, TagExt, TagType};
	///
	/// let mut tag = Tag::new(TagType::Id3v2);
	/// tag.set_album(String::from("Album"));
	///
	/// // ID3v2 supports the album tag
	/// assert_eq!(tag.len(), 1);
	///
	/// // But AIFF text chunks do not, the item will be lost
	/// tag.re_map(TagType::AiffText);
	/// assert!(tag.is_empty());
	/// ```
	///
	/// [`GlobalOptions::preserve_format_specific_items`]: crate::config::GlobalOptions::preserve_format_specific_items
	pub fn re_map(&mut self, tag_type: TagType) {
		if let Some(companion_tag) = self.companion_tag.take() {
			log::warn!("Discarding format-specific items due to remap");
			drop(companion_tag);
		}

		self.retain(|i| i.re_map(tag_type));
		self.tag_type = tag_type
	}

	/// Check if the tag contains any format-specific items
	///
	/// See [`GlobalOptions::preserve_format_specific_items`].
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::tag::{Accessor, Tag, TagExt, TagType};
	///
	/// let mut tag = Tag::new(TagType::Id3v2);
	/// tag.set_album(String::from("Album"));
	///
	/// // We cannot create a tag with format-specific items.
	/// // This must come from a conversion, such as `Id3v2Tag` -> `Tag`
	/// assert!(!tag.has_format_specific_items());
	/// ```
	///
	/// [`GlobalOptions::preserve_format_specific_items`]: crate::config::GlobalOptions::preserve_format_specific_items
	pub fn has_format_specific_items(&self) -> bool {
		self.companion_tag.is_some()
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
	pub fn items(&self) -> impl ExactSizeIterator<Item = &TagItem> + Clone {
		self.items.iter()
	}

	/// Returns all [`Popularimeter`] ratings
	///
	/// # Examples
	///
	/// ```
	/// use lofty::tag::items::popularimeter::{Popularimeter, StarRating};
	/// use lofty::tag::{ItemKey, Tag, TagType};
	///
	/// let mut tag = Tag::new(TagType::Id3v2);
	///
	/// /// Create a new popularimeter
	/// let star_rating = StarRating::Three;
	/// let play_count = 5;
	/// let popularimeter = Popularimeter::musicbee(star_rating, play_count);
	///
	/// /// Insert the popularimeter as text
	/// tag.insert_text(ItemKey::Popularimeter, popularimeter.to_string());
	///
	/// /// Fetch all ratings
	/// let mut ratings = tag.ratings();
	/// let first_rating = ratings.next().expect("should exist");
	/// assert!(ratings.next().is_none());
	///
	/// assert_eq!(first_rating.email(), popularimeter.email());
	/// assert_eq!(first_rating.rating, popularimeter.rating);
	/// assert_eq!(first_rating.play_counter, popularimeter.play_counter);
	/// ```
	pub fn ratings(&self) -> impl Iterator<Item = Popularimeter<'static>> + Clone {
		self.get_strings(ItemKey::Popularimeter)
			.filter_map(|i| Popularimeter::from_str(i).ok())
	}

	/// Returns a reference to a [`TagItem`] matching an [`ItemKey`]
	pub fn get(&self, item_key: ItemKey) -> Option<&TagItem> {
		self.items.iter().find(|i| i.item_key == item_key)
	}

	/// Get a string value from an [`ItemKey`]
	pub fn get_string(&self, item_key: ItemKey) -> Option<&str> {
		if let Some(ItemValue::Text(ret)) = self.get(item_key).map(TagItem::value) {
			return Some(ret);
		}

		None
	}

	fn get_u32_from_string(&self, key: ItemKey) -> Option<u32> {
		let i = self.get_string(key)?;
		i.parse::<u32>().ok()
	}

	/// Gets a byte slice from an [`ItemKey`]
	///
	/// Use `convert` to convert [`ItemValue::Text`] and [`ItemValue::Locator`] to byte slices
	pub fn get_binary(&self, item_key: ItemKey, convert: bool) -> Option<&[u8]> {
		if let Some(item) = self.get(item_key) {
			match item.value() {
				ItemValue::Text(text) | ItemValue::Locator(text) if convert => {
					return Some(text.as_bytes());
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
	pub fn insert(&mut self, item: TagItem) -> bool {
		if item.re_map(self.tag_type) {
			self.insert_unchecked(item);
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
	pub fn insert_unchecked(&mut self, item: TagItem) {
		self.retain(|i| i.item_key != item.item_key);
		self.items.push(item);
	}

	/// Append a [`TagItem`] to the tag
	///
	/// This will not remove any items of the same [`ItemKey`], unlike [`Tag::insert`]
	///
	/// NOTE: This **will** verify an [`ItemKey`] mapping exists for the target [`TagType`]
	///
	/// Multiple items of the same [`ItemKey`] are not valid in all formats, in which case
	/// the first available item will be used.
	///
	/// This will return `true` if the item was pushed.
	pub fn push(&mut self, item: TagItem) -> bool {
		if item.re_map(self.tag_type) {
			self.items.push(item);
			return true;
		}

		false
	}

	/// Append a [`TagItem`] to the tag
	///
	/// Notes: See [`Tag::push()`] and the notes of [`Tag::insert_unchecked()`]
	pub fn push_unchecked(&mut self, item: TagItem) {
		self.items.push(item);
	}

	/// An alias for [`Tag::insert`] that doesn't require the user to create a [`TagItem`]
	///
	/// NOTE: This will replace any existing item with `item_key`. See [`Tag::insert`]
	pub fn insert_text(&mut self, item_key: ItemKey, text: String) -> bool {
		self.insert(TagItem::new(item_key, ItemValue::Text(text)))
	}

	/// Removes all items with the specified [`ItemKey`], and returns them
	///
	/// See also: [take_filter()](Self::take_filter)
	pub fn take(&mut self, key: ItemKey) -> impl Iterator<Item = TagItem> + use<'_> {
		self.take_filter(key, |_| true)
	}

	/// Removes selected items with the specified [`ItemKey`], and returns them
	///
	/// Only takes items for which `filter()` returns `true`. All other items are retained.
	///
	/// Returns the selected items in order and preserves the ordering of the remaining items.
	///
	/// # Examples
	///
	/// ```
	/// use lofty::tag::{ItemKey, ItemValue, Tag, TagItem, TagType};
	///
	/// let mut tag = Tag::new(TagType::Id3v2);
	/// tag.push(TagItem::new(
	/// 	ItemKey::Comment,
	/// 	ItemValue::Text("comment without description".to_owned()),
	/// ));
	/// let mut item = TagItem::new(
	/// 	ItemKey::Comment,
	/// 	ItemValue::Text("comment with description".to_owned()),
	/// );
	/// item.set_description("description".to_owned());
	/// tag.push(item);
	/// assert_eq!(tag.get_strings(ItemKey::Comment).count(), 2);
	///
	/// // Extract all comment items with an empty description.
	/// let comments = tag
	/// 	.take_filter(ItemKey::Comment, |item| item.description().is_empty())
	/// 	.filter_map(|item| item.into_value().into_string())
	/// 	.collect::<Vec<_>>();
	/// assert_eq!(comments, vec!["comment without description".to_owned()]);
	///
	/// // The comments that didn't match the filter are still present.
	/// assert_eq!(tag.get_strings(ItemKey::Comment).count(), 1);
	/// ```
	pub fn take_filter<F>(
		&mut self,
		key: ItemKey,
		mut filter: F,
	) -> impl Iterator<Item = TagItem> + use<'_, F>
	where
		F: FnMut(&TagItem) -> bool,
	{
		// TODO: drain_filter
		let mut split_idx = 0;

		for read_idx in 0..self.items.len() {
			let item = &self.items[read_idx];
			if item.key() == key && filter(item) {
				self.items.swap(split_idx, read_idx);
				split_idx += 1;
			}
		}

		self.items.drain(..split_idx)
	}

	/// Removes all items with the specified [`ItemKey`], and filters them through [`ItemValue::into_string`]
	pub fn take_strings(&mut self, key: ItemKey) -> impl Iterator<Item = String> + use<'_> {
		self.take(key).filter_map(|i| i.item_value.into_string())
	}

	/// Returns references to all [`TagItem`]s with the specified key
	pub fn get_items(&self, key: ItemKey) -> impl Iterator<Item = &TagItem> + Clone {
		self.items.iter().filter(move |i| i.key() == key)
	}

	/// Returns references to all texts of [`TagItem`]s with the specified key, and [`ItemValue::Text`]
	pub fn get_strings(&self, key: ItemKey) -> impl Iterator<Item = &str> + Clone {
		self.items.iter().filter_map(move |i| {
			if i.key() == key {
				i.value().text()
			} else {
				None
			}
		})
	}

	/// Returns references to all locators of [`TagItem`]s with the specified key, and [`ItemValue::Locator`]
	pub fn get_locators(&self, key: ItemKey) -> impl Iterator<Item = &str> + Clone {
		self.items.iter().filter_map(move |i| {
			if i.key() == key {
				i.value().locator()
			} else {
				None
			}
		})
	}

	/// Returns references to all bytes of [`TagItem`]s with the specified key, and [`ItemValue::Binary`]
	pub fn get_bytes(&self, key: ItemKey) -> impl Iterator<Item = &[u8]> + Clone {
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
	pub fn remove_key(&mut self, key: ItemKey) {
		self.items.retain(|i| i.key() != key)
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

	/// Remove all items with empty values
	pub fn remove_empty(&mut self) {
		self.items.retain(|item| !item.value().is_empty());
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
	/// use lofty::picture::{MimeType, Picture, PictureType};
	/// use lofty::tag::{Tag, TagType};
	///
	/// let mut tag = Tag::new(TagType::Id3v2);
	///
	/// // Add a front cover
	/// let front_cover = Picture::unchecked(Vec::new())
	/// 	.pic_type(PictureType::CoverFront)
	/// 	.mime_type(MimeType::Png)
	/// 	.build();
	/// tag.push_picture(front_cover);
	///
	/// assert_eq!(tag.pictures().len(), 1);
	/// assert_eq!(tag.pictures()[0].pic_type(), PictureType::CoverFront);
	///
	/// // Replace the front cover with a back cover
	/// let back_cover = Picture::unchecked(Vec::new())
	/// 	.pic_type(PictureType::CoverBack)
	/// 	.mime_type(MimeType::Png)
	/// 	.build();
	/// tag.set_picture(0, back_cover);
	///
	/// assert_eq!(tag.pictures().len(), 1);
	/// assert_eq!(tag.pictures()[0].pic_type(), PictureType::CoverBack);
	///
	/// // Use an out of bounds index
	/// let another_picture = Picture::unchecked(Vec::new())
	/// 	.pic_type(PictureType::Band)
	/// 	.mime_type(MimeType::Png)
	/// 	.build();
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
	/// use lofty::picture::{MimeType, Picture, PictureType};
	/// use lofty::tag::{Tag, TagType};
	///
	/// let mut tag = Tag::new(TagType::Id3v2);
	///
	/// let picture = Picture::unchecked(Vec::new())
	/// 	.pic_type(PictureType::CoverFront)
	/// 	.mime_type(MimeType::Png)
	/// 	.build();
	///
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
	type RefKey<'a> = ItemKey;

	#[inline]
	fn tag_type(&self) -> TagType {
		self.tag_type
	}

	fn len(&self) -> usize {
		self.items.len() + self.pictures.len()
	}

	fn contains<'a>(&'a self, key: Self::RefKey<'a>) -> bool {
		self.items.iter().any(|item| item.key() == key)
	}

	fn is_empty(&self) -> bool {
		self.items.is_empty() && self.pictures.is_empty()
	}

	/// Save the `Tag` to a [`FileLike`]
	///
	/// # Errors
	///
	/// * A [`FileType`](crate::file::FileType) couldn't be determined from the File
	/// * Attempting to write a tag to a format that does not support it. See [`FileType::tag_support()`](crate::file::FileType::tag_support)
	fn save_to<F>(
		&self,
		file: &mut F,
		write_options: WriteOptions,
	) -> std::result::Result<(), Self::Err>
	where
		F: FileLike,
		LoftyError: From<<F as Truncate>::Error>,
		LoftyError: From<<F as Length>::Error>,
	{
		let probe = Probe::new(file).guess_file_type()?;

		match probe.file_type() {
			Some(file_type) => {
				if !file_type.tag_support(self.tag_type).is_writable() {
					err!(UnsupportedTag);
				}

				utils::write_tag(self, probe.into_inner(), file_type, write_options)
			},
			None => err!(UnknownFormat),
		}
	}

	fn dump_to<W: Write>(&self, writer: &mut W, write_options: WriteOptions) -> Result<()> {
		utils::dump_tag(self, writer, write_options)
	}

	/// Remove a tag from a [`Path`]
	///
	/// # Errors
	///
	/// See [`TagType::remove_from`]
	fn remove_from_path<P: AsRef<Path>>(&self, path: P) -> std::result::Result<(), Self::Err> {
		self.tag_type.remove_from_path(path)
	}

	/// Remove a tag from a [`FileLike`]
	///
	/// # Errors
	///
	/// See [`TagType::remove_from`]
	fn remove_from<F>(&self, file: &mut F) -> std::result::Result<(), Self::Err>
	where
		F: FileLike,
		LoftyError: From<<F as Truncate>::Error>,
		LoftyError: From<<F as Length>::Error>,
	{
		self.tag_type.remove_from(file)
	}

	fn clear(&mut self) {
		self.items.clear();
		self.pictures.clear();
	}
}

#[derive(Debug, Clone, Default)]
#[allow(missing_docs)]
pub struct SplitTagRemainder;

impl SplitTag for Tag {
	type Remainder = SplitTagRemainder;

	fn split_tag(self) -> (Self::Remainder, Self) {
		(SplitTagRemainder, self)
	}
}

impl MergeTag for SplitTagRemainder {
	type Merged = Tag;

	fn merge_tag(self, tag: Tag) -> Self::Merged {
		tag
	}
}

#[cfg(test)]
mod tests {
	use super::try_parse_timestamp;
	use crate::config::WriteOptions;
	use crate::picture::{Picture, PictureType};
	use crate::prelude::*;
	use crate::tag::utils::test_utils::read_path;
	use crate::tag::{Tag, TagType};

	use std::io::{Seek, Write};
	use std::process::Command;

	#[test_log::test]
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
		tag.save_to(temp_file.as_file_mut(), WriteOptions::default())
			.unwrap();

		let cmd_output = Command::new("ffprobe")
			.arg(temp_file.path().to_str().unwrap())
			.output()
			.unwrap();

		assert!(cmd_output.status.success());

		let stdout = String::from_utf8(cmd_output.stdout).unwrap();

		assert!(!stdout.contains("CRC mismatch!"));
		assert!(
			!stdout.contains("Header processing failed: Invalid data found when processing input")
		);
	}

	#[test_log::test]
	fn issue_130_huge_picture() {
		// Verify we have opus-tools available, otherwise skip
		match Command::new("opusinfo").output() {
			Err(e) if matches!(e.kind(), std::io::ErrorKind::NotFound) => {
				eprintln!("Skipping test, `opus-tools` is not installed!");
				return;
			},
			Err(e) => panic!("{}", e),
			_ => {},
		}

		let file_contents = read_path("tests/files/assets/minimal/full_test.opus");
		let mut temp_file = tempfile::NamedTempFile::new().unwrap();
		temp_file.write_all(&file_contents).unwrap();
		temp_file.rewind().unwrap();

		let mut tag = Tag::new(TagType::VorbisComments);

		// 81KB picture, which is big enough to surpass the maximum page size
		let mut picture =
			Picture::from_reader(&mut &*read_path("tests/files/assets/issue_37.jpg")).unwrap();
		picture.set_pic_type(PictureType::CoverFront);

		tag.push_picture(picture);
		tag.save_to(temp_file.as_file_mut(), WriteOptions::default())
			.unwrap();

		let cmd_output = Command::new("opusinfo")
			.arg(temp_file.path().to_str().unwrap())
			.output()
			.unwrap();

		let stdout = String::from_utf8(cmd_output.stdout).unwrap();

		assert!(cmd_output.status.success(), "{stdout}");
		assert!(!stdout.contains("WARNING:"));
	}

	#[test_log::test]
	fn should_preserve_empty_title() {
		let mut tag = Tag::new(TagType::Id3v2);
		tag.set_title(String::from("Foo title"));

		assert_eq!(tag.title().as_deref(), Some("Foo title"));

		tag.set_title(String::new());
		assert_eq!(tag.title().as_deref(), Some(""));

		tag.remove_title();
		assert_eq!(tag.title(), None);
	}

	#[test_log::test]
	fn should_not_parse_year_from_less_than_4_digits() {
		assert!(try_parse_timestamp("198").is_none());
		assert!(try_parse_timestamp("19").is_none());
		assert!(try_parse_timestamp("1").is_none());
	}
}
