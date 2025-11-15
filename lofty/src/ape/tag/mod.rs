pub(crate) mod item;
pub(crate) mod read;
mod write;

use crate::ape::tag::item::{ApeItem, ApeItemRef};
use crate::config::WriteOptions;
use crate::error::{LoftyError, Result};
use crate::id3::v2::util::pairs::{NUMBER_PAIR_KEYS, format_number_pair, set_number};
use crate::tag::item::ItemValueRef;
use crate::tag::items::Timestamp;
use crate::tag::{
	Accessor, ItemKey, ItemValue, MergeTag, SplitTag, Tag, TagExt, TagItem, TagType,
	try_parse_timestamp,
};
use crate::util::flag_item;
use crate::util::io::{FileLike, Truncate};

use std::borrow::Cow;
use std::io::Write;
use std::ops::Deref;

use lofty_attr::tag;

macro_rules! impl_accessor {
	($($name:ident => $($key:literal)|+;)+) => {
		paste::paste! {
			$(
				fn $name(&self) -> Option<Cow<'_, str>> {
					$(
						if let Some(i) = self.get($key) {
							if let ItemValue::Text(val) = i.value() {
								return Some(Cow::Borrowed(val));
							}
						}
					)+

					None
				}

				fn [<set_ $name>](&mut self, value: String) {
					self.insert(ApeItem {
						read_only: false,
						key: String::from(crate::tag::item::first_key!($($key)|*)),
						value: ItemValue::Text(value)
					})
				}

				fn [<remove_ $name>](&mut self) {
					$(
						self.remove($key);
					)+
				}
			)+
		}
	}
}

/// ## Item storage
///
/// `APE` isn't a very strict format. An [`ApeItem`] only restricted by its name, meaning it can use
/// a normal [`ItemValue`](crate::tag::ItemValue) unlike other formats.
///
/// Pictures are stored as [`ItemValue::Binary`](crate::tag::ItemValue::Binary), and can be converted with
/// [`Picture::from_ape_bytes()`]. For the appropriate item keys, see [`APE_PICTURE_TYPES`].
///
/// ## Conversions
///
/// ### To `Tag`
///
/// Any [`ApeItem`] with an [`ItemKey`] mapping will have a 1:1 conversion to [`TagItem`].
///
/// ### From `Tag`
///
/// Converting [`Tag`]s into `ApeTag` is *mostly* seamless.
///
/// #### Items
///
/// As long at the [`ItemKey`] has a mapping for APE, it can be converted to an [`ApeItem`].
///
/// #### Pictures
///
/// When converting pictures, any of type [`PictureType::Undefined`](crate::picture::PictureType::Undefined) will be discarded.
/// Since APE doesn't have dedicated item types for pictures like other formats (e.g. [Id3v2Tag]), pictures
/// **must** be disambiguated by their [`PictureType`](crate::picture::PictureType).
///
/// [`Picture::from_ape_bytes()`]: crate::picture::Picture::from_ape_bytes
/// [`APE_PICTURE_TYPES`]: crate::ape::APE_PICTURE_TYPES
#[derive(Default, Debug, PartialEq, Eq, Clone)]
#[tag(
	description = "An `APE` tag",
	supported_formats(Ape, Mpeg, Mpc, WavPack)
)]
pub struct ApeTag {
	/// Whether or not to mark the tag as read only
	pub read_only: bool,
	pub(super) items: Vec<ApeItem>,
}

impl ApeTag {
	/// Create a new empty `ApeTag`
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ape::ApeTag;
	/// use lofty::tag::TagExt;
	///
	/// let ape_tag = ApeTag::new();
	/// assert!(ape_tag.is_empty());
	/// ```
	pub fn new() -> Self {
		Self::default()
	}

	/// Get an [`ApeItem`] by key
	///
	/// NOTE: While `APE` items are supposed to be case-sensitive,
	/// this rule is rarely followed, so this will ignore case when searching.
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ape::ApeTag;
	/// use lofty::tag::Accessor;
	///
	/// let mut ape_tag = ApeTag::new();
	/// ape_tag.set_title(String::from("Foo title"));
	///
	/// // Get the title by its key
	/// let title = ape_tag.get("Title");
	/// assert!(title.is_some());
	/// ```
	pub fn get(&self, key: &str) -> Option<&ApeItem> {
		self.items
			.iter()
			.find(|i| i.key().eq_ignore_ascii_case(key))
	}

	/// Insert an [`ApeItem`]
	///
	/// This will remove any item with the same key prior to insertion
	pub fn insert(&mut self, value: ApeItem) {
		self.remove(value.key());
		self.items.push(value);
	}

	/// Remove an [`ApeItem`] by key
	///
	/// NOTE: Like [`ApeTag::get`], this is not case-sensitive
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ape::ApeTag;
	/// use lofty::tag::Accessor;
	///
	/// let mut ape_tag = ApeTag::new();
	/// ape_tag.set_title(String::from("Foo title"));
	///
	/// // Get the title by its key
	/// let title = ape_tag.get("Title");
	/// assert!(title.is_some());
	///
	/// // Remove the title
	/// ape_tag.remove("Title");
	///
	/// let title = ape_tag.get("Title");
	/// assert!(title.is_none());
	/// ```
	pub fn remove(&mut self, key: &str) {
		self.items.retain(|i| !i.key().eq_ignore_ascii_case(key));
	}

	fn insert_item(&mut self, item: TagItem) {
		match item.key() {
			ItemKey::TrackNumber => set_number(&item, |number| self.set_track(number)),
			ItemKey::TrackTotal => set_number(&item, |number| self.set_track_total(number)),
			ItemKey::DiscNumber => set_number(&item, |number| self.set_disk(number)),
			ItemKey::DiscTotal => set_number(&item, |number| self.set_disk_total(number)),

			// Normalize flag items
			ItemKey::FlagCompilation => {
				let Some(text) = item.item_value.text() else {
					return;
				};

				let Some(flag) = flag_item(text) else {
					return;
				};

				let value = u8::from(flag).to_string();
				self.insert(ApeItem::text("Compilation", value));
			},
			_ => {
				if let Ok(item) = item.try_into() {
					self.insert(item);
				}
			},
		}
	}

	fn split_num_pair(&self, key: &str) -> (Option<u32>, Option<u32>) {
		if let Some(ApeItem {
			value: ItemValue::Text(text),
			..
		}) = self.get(key)
		{
			let mut split = text.split('/').flat_map(str::parse::<u32>);
			return (split.next(), split.next());
		}

		(None, None)
	}

	fn insert_number_pair(&mut self, key: &'static str, number: Option<u32>, total: Option<u32>) {
		if let Some(value) = format_number_pair(number, total) {
			self.insert(ApeItem::text(key, value));
		} else {
			log::warn!("{key} is not set. number: {number:?}, total: {total:?}");
		}
	}
}

impl IntoIterator for ApeTag {
	type Item = ApeItem;
	type IntoIter = std::vec::IntoIter<Self::Item>;

	fn into_iter(self) -> Self::IntoIter {
		self.items.into_iter()
	}
}

impl<'a> IntoIterator for &'a ApeTag {
	type Item = &'a ApeItem;
	type IntoIter = std::slice::Iter<'a, ApeItem>;

	fn into_iter(self) -> Self::IntoIter {
		self.items.iter()
	}
}

impl Accessor for ApeTag {
	impl_accessor!(
		artist  => "Artist";
		title   => "Title";
		album   => "Album";
		genre   => "GENRE";
		comment => "Comment";
	);

	fn track(&self) -> Option<u32> {
		self.split_num_pair("Track").0
	}

	fn set_track(&mut self, value: u32) {
		self.insert_number_pair("Track", Some(value), self.track_total());
	}

	fn remove_track(&mut self) {
		self.remove("Track");
	}

	fn track_total(&self) -> Option<u32> {
		self.split_num_pair("Track").1
	}

	fn set_track_total(&mut self, value: u32) {
		self.insert_number_pair("Track", self.track(), Some(value));
	}

	fn remove_track_total(&mut self) {
		let existing_track_number = self.track();
		self.remove("Track");

		if let Some(track) = existing_track_number {
			self.insert(ApeItem::text("Track", track.to_string()));
		}
	}

	fn disk(&self) -> Option<u32> {
		self.split_num_pair("Disc").0
	}

	fn set_disk(&mut self, value: u32) {
		self.insert_number_pair("Disc", Some(value), self.disk_total());
	}

	fn remove_disk(&mut self) {
		self.remove("Disc");
	}

	fn disk_total(&self) -> Option<u32> {
		self.split_num_pair("Disc").1
	}

	fn set_disk_total(&mut self, value: u32) {
		self.insert_number_pair("Disc", self.disk(), Some(value));
	}

	fn remove_disk_total(&mut self) {
		let existing_disk_number = self.disk();
		self.remove("Disc");

		if let Some(disk) = existing_disk_number {
			self.insert(ApeItem::text("Disc", disk.to_string()));
		}
	}

	// For some reason, the ecosystem agreed on the key "Year", even for full date strings.
	fn date(&self) -> Option<Timestamp> {
		if let Some(ApeItem {
			value: ItemValue::Text(text),
			..
		}) = self.get("Year")
		{
			return try_parse_timestamp(text);
		}

		None
	}

	fn set_date(&mut self, value: Timestamp) {
		self.insert(ApeItem::text("Year", value.to_string()));
	}

	fn remove_date(&mut self) {
		self.remove("Year");
	}
}

impl TagExt for ApeTag {
	type Err = LoftyError;
	type RefKey<'a> = &'a str;

	#[inline]
	fn tag_type(&self) -> TagType {
		TagType::Ape
	}

	fn len(&self) -> usize {
		self.items.len()
	}

	fn contains<'a>(&'a self, key: Self::RefKey<'a>) -> bool {
		self.items.iter().any(|i| i.key().eq_ignore_ascii_case(key))
	}

	fn is_empty(&self) -> bool {
		self.items.is_empty()
	}

	/// Write an `APE` tag to a file
	///
	/// # Errors
	///
	/// * Attempting to write the tag to a format that does not support it
	/// * An existing tag has an invalid size
	fn save_to<F>(
		&self,
		file: &mut F,
		write_options: WriteOptions,
	) -> std::result::Result<(), Self::Err>
	where
		F: FileLike,
		LoftyError: From<<F as Truncate>::Error>,
	{
		ApeTagRef {
			read_only: self.read_only,
			items: self.items.iter().map(Into::into),
		}
		.write_to(file, write_options)
	}

	/// Dumps the tag to a writer
	///
	/// # Errors
	///
	/// * [`std::io::Error`]
	fn dump_to<W: Write>(
		&self,
		writer: &mut W,
		write_options: WriteOptions,
	) -> std::result::Result<(), Self::Err> {
		ApeTagRef {
			read_only: self.read_only,
			items: self.items.iter().map(Into::into),
		}
		.dump_to(writer, write_options)
	}

	fn clear(&mut self) {
		self.items.clear();
	}
}

#[derive(Debug, Clone, Default)]
pub struct SplitTagRemainder(ApeTag);

impl From<SplitTagRemainder> for ApeTag {
	fn from(from: SplitTagRemainder) -> Self {
		from.0
	}
}

impl Deref for SplitTagRemainder {
	type Target = ApeTag;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl SplitTag for ApeTag {
	type Remainder = SplitTagRemainder;

	fn split_tag(mut self) -> (Self::Remainder, Tag) {
		fn split_pair(
			content: &str,
			tag: &mut Tag,
			current_key: ItemKey,
			total_key: ItemKey,
		) -> Option<()> {
			let mut split = content.splitn(2, '/');
			let current = split.next()?.to_string();
			tag.items
				.push(TagItem::new(current_key, ItemValue::Text(current)));

			if let Some(total) = split.next() {
				tag.items
					.push(TagItem::new(total_key, ItemValue::Text(total.to_string())))
			}

			Some(())
		}

		let mut tag = Tag::new(TagType::Ape);

		self.items.retain_mut(|item| {
			let Some(item_key) = ItemKey::from_key(TagType::Ape, item.key()) else {
				return true;
			};

			// The text pairs need some special treatment
			match (item_key, item.value()) {
				(ItemKey::TrackNumber | ItemKey::TrackTotal, ItemValue::Text(val))
					if split_pair(val, &mut tag, ItemKey::TrackNumber, ItemKey::TrackTotal)
						.is_some() =>
				{
					*item = ApeItem::EMPTY;
					false // Item consumed
				},
				(ItemKey::DiscNumber | ItemKey::DiscTotal, ItemValue::Text(val))
					if split_pair(val, &mut tag, ItemKey::DiscNumber, ItemKey::DiscTotal)
						.is_some() =>
				{
					*item = ApeItem::EMPTY;
					false // Item consumed
				},
				(ItemKey::MovementNumber | ItemKey::MovementTotal, ItemValue::Text(val))
					if split_pair(
						val,
						&mut tag,
						ItemKey::MovementNumber,
						ItemKey::MovementTotal,
					)
					.is_some() =>
				{
					*item = ApeItem::EMPTY;
					false // Item consumed
				},
				(k, _) => {
					let item = std::mem::replace(item, ApeItem::EMPTY);
					tag.items.push(TagItem::new(k, item.value));
					false // Item consumed
				},
			}
		});

		(SplitTagRemainder(self), tag)
	}
}

impl MergeTag for SplitTagRemainder {
	type Merged = ApeTag;

	fn merge_tag(self, tag: Tag) -> Self::Merged {
		let Self(mut merged) = self;

		for item in tag.items {
			merged.insert_item(item);
		}

		for pic in tag.pictures {
			if let Some(key) = pic.pic_type.as_ape_key() {
				if let Ok(item) =
					ApeItem::new(key.to_string(), ItemValue::Binary(pic.as_ape_bytes()))
				{
					merged.insert(item)
				}
			}
		}

		merged
	}
}

impl From<ApeTag> for Tag {
	fn from(input: ApeTag) -> Self {
		input.split_tag().1
	}
}

impl From<Tag> for ApeTag {
	fn from(input: Tag) -> Self {
		SplitTagRemainder::default().merge_tag(input)
	}
}

pub(crate) struct ApeTagRef<'a, I>
where
	I: Iterator<Item = ApeItemRef<'a>>,
{
	pub(crate) read_only: bool,
	pub(crate) items: I,
}

impl<'a, I> ApeTagRef<'a, I>
where
	I: Iterator<Item = ApeItemRef<'a>>,
{
	pub(crate) fn write_to<F>(&mut self, file: &mut F, write_options: WriteOptions) -> Result<()>
	where
		F: FileLike,
		LoftyError: From<<F as Truncate>::Error>,
	{
		write::write_to(file, self, write_options)
	}

	pub(crate) fn dump_to<W: Write>(
		&mut self,
		writer: &mut W,
		write_options: WriteOptions,
	) -> Result<()> {
		let temp = write::create_ape_tag(self, std::iter::empty(), write_options)?;
		writer.write_all(&temp)?;

		Ok(())
	}
}

pub(crate) fn tagitems_into_ape(tag: &Tag) -> impl Iterator<Item = ApeItemRef<'_>> {
	fn create_apeitemref_for_number_pair<'a>(
		number: Option<&str>,
		total: Option<&str>,
		key: &'a str,
	) -> Option<ApeItemRef<'a>> {
		format_number_pair(number, total).map(|value| ApeItemRef {
			read_only: false,
			key,
			value: ItemValueRef::Text(Cow::Owned(value)),
		})
	}

	tag.items()
		.filter(|item| !NUMBER_PAIR_KEYS.contains(&item.key()))
		.filter_map(|i| {
			i.key().map_key(TagType::Ape).map(|key| ApeItemRef {
				read_only: false,
				key,
				value: (&i.item_value).into(),
			})
		})
		.chain(create_apeitemref_for_number_pair(
			tag.get_string(ItemKey::TrackNumber),
			tag.get_string(ItemKey::TrackTotal),
			"Track",
		))
		.chain(create_apeitemref_for_number_pair(
			tag.get_string(ItemKey::DiscNumber),
			tag.get_string(ItemKey::DiscTotal),
			"Disc",
		))
}

#[cfg(test)]
mod tests {
	use crate::ape::{ApeItem, ApeTag};
	use crate::config::{ParseOptions, WriteOptions};
	use crate::id3::v2::util::pairs::DEFAULT_NUMBER_IN_PAIR;
	use crate::prelude::*;
	use crate::tag::{ItemValue, Tag, TagItem, TagType};

	use crate::picture::{MimeType, Picture, PictureType};
	use std::io::Cursor;

	#[test_log::test]
	fn parse_ape() {
		let mut expected_tag = ApeTag::default();

		let title_item = ApeItem::new(
			String::from("TITLE"),
			ItemValue::Text(String::from("Foo title")),
		)
		.unwrap();

		let artist_item = ApeItem::new(
			String::from("ARTIST"),
			ItemValue::Text(String::from("Bar artist")),
		)
		.unwrap();

		let album_item = ApeItem::new(
			String::from("ALBUM"),
			ItemValue::Text(String::from("Baz album")),
		)
		.unwrap();

		let comment_item = ApeItem::new(
			String::from("COMMENT"),
			ItemValue::Text(String::from("Qux comment")),
		)
		.unwrap();

		let year_item =
			ApeItem::new(String::from("YEAR"), ItemValue::Text(String::from("1984"))).unwrap();

		let track_number_item =
			ApeItem::new(String::from("TRACK"), ItemValue::Text(String::from("1"))).unwrap();

		let genre_item = ApeItem::new(
			String::from("GENRE"),
			ItemValue::Text(String::from("Classical")),
		)
		.unwrap();

		expected_tag.insert(title_item);
		expected_tag.insert(artist_item);
		expected_tag.insert(album_item);
		expected_tag.insert(comment_item);
		expected_tag.insert(year_item);
		expected_tag.insert(track_number_item);
		expected_tag.insert(genre_item);

		let tag = crate::tag::utils::test_utils::read_path("tests/tags/assets/test.apev2");
		let mut reader = Cursor::new(tag);

		let (Some(parsed_tag), _) =
			crate::ape::tag::read::read_ape_tag(&mut reader, false, ParseOptions::new()).unwrap()
		else {
			unreachable!();
		};

		assert_eq!(expected_tag.len(), parsed_tag.len());

		for item in &expected_tag.items {
			assert!(parsed_tag.items.contains(item));
		}
	}

	#[test_log::test]
	fn ape_re_read() {
		let tag_bytes = crate::tag::utils::test_utils::read_path("tests/tags/assets/test.apev2");
		let mut reader = Cursor::new(tag_bytes);

		let (Some(parsed_tag), _) =
			crate::ape::tag::read::read_ape_tag(&mut reader, false, ParseOptions::new()).unwrap()
		else {
			unreachable!();
		};

		let mut writer = Vec::new();
		parsed_tag
			.dump_to(&mut writer, WriteOptions::default())
			.unwrap();

		let mut temp_reader = Cursor::new(writer);

		let (Some(temp_parsed_tag), _) =
			crate::ape::tag::read::read_ape_tag(&mut temp_reader, false, ParseOptions::new())
				.unwrap()
		else {
			unreachable!();
		};

		assert_eq!(parsed_tag, temp_parsed_tag);
	}

	#[test_log::test]
	fn ape_to_tag() {
		let tag_bytes = crate::tag::utils::test_utils::read_path("tests/tags/assets/test.apev2");
		let mut reader = Cursor::new(tag_bytes);

		let (Some(ape), _) =
			crate::ape::tag::read::read_ape_tag(&mut reader, false, ParseOptions::new()).unwrap()
		else {
			unreachable!();
		};

		let tag: Tag = ape.into();

		crate::tag::utils::test_utils::verify_tag(&tag, true, true);
	}

	#[test_log::test]
	fn tag_to_ape() {
		fn verify_key(tag: &ApeTag, key: &str, expected_val: &str) {
			assert_eq!(
				tag.get(key).map(ApeItem::value),
				Some(&ItemValue::Text(String::from(expected_val)))
			);
		}

		let tag = crate::tag::utils::test_utils::create_tag(TagType::Ape);

		let ape_tag: ApeTag = tag.into();

		verify_key(&ape_tag, "Title", "Foo title");
		verify_key(&ape_tag, "Artist", "Bar artist");
		verify_key(&ape_tag, "Album", "Baz album");
		verify_key(&ape_tag, "Comment", "Qux comment");
		verify_key(&ape_tag, "Track", "1");
		verify_key(&ape_tag, "Genre", "Classical");
	}

	#[test_log::test]
	fn set_track() {
		let mut ape = ApeTag::default();
		let track = 1;

		ape.set_track(track);

		assert_eq!(ape.track().unwrap(), track);
		assert!(ape.track_total().is_none());
	}

	#[test_log::test]
	fn set_track_total() {
		let mut ape = ApeTag::default();
		let track_total = 2;

		ape.set_track_total(track_total);

		assert_eq!(ape.track().unwrap(), DEFAULT_NUMBER_IN_PAIR);
		assert_eq!(ape.track_total().unwrap(), track_total);
	}

	#[test_log::test]
	fn set_track_and_track_total() {
		let mut ape = ApeTag::default();
		let track = 1;
		let track_total = 2;

		ape.set_track(track);
		ape.set_track_total(track_total);

		assert_eq!(ape.track().unwrap(), track);
		assert_eq!(ape.track_total().unwrap(), track_total);
	}

	#[test_log::test]
	fn set_track_total_and_track() {
		let mut ape = ApeTag::default();
		let track_total = 2;
		let track = 1;

		ape.set_track_total(track_total);
		ape.set_track(track);

		assert_eq!(ape.track_total().unwrap(), track_total);
		assert_eq!(ape.track().unwrap(), track);
	}

	#[test_log::test]
	fn set_disk() {
		let mut ape = ApeTag::default();
		let disk = 1;

		ape.set_disk(disk);

		assert_eq!(ape.disk().unwrap(), disk);
		assert!(ape.disk_total().is_none());
	}

	#[test_log::test]
	fn set_disk_total() {
		let mut ape = ApeTag::default();
		let disk_total = 2;

		ape.set_disk_total(disk_total);

		assert_eq!(ape.disk().unwrap(), DEFAULT_NUMBER_IN_PAIR);
		assert_eq!(ape.disk_total().unwrap(), disk_total);
	}

	#[test_log::test]
	fn set_disk_and_disk_total() {
		let mut ape = ApeTag::default();
		let disk = 1;
		let disk_total = 2;

		ape.set_disk(disk);
		ape.set_disk_total(disk_total);

		assert_eq!(ape.disk().unwrap(), disk);
		assert_eq!(ape.disk_total().unwrap(), disk_total);
	}

	#[test_log::test]
	fn set_disk_total_and_disk() {
		let mut ape = ApeTag::default();
		let disk_total = 2;
		let disk = 1;

		ape.set_disk_total(disk_total);
		ape.set_disk(disk);

		assert_eq!(ape.disk_total().unwrap(), disk_total);
		assert_eq!(ape.disk().unwrap(), disk);
	}

	#[test_log::test]
	fn track_number_tag_to_ape() {
		let track_number = 1;

		let mut tag = Tag::new(TagType::Ape);

		tag.push(TagItem::new(
			ItemKey::TrackNumber,
			ItemValue::Text(track_number.to_string()),
		));

		let tag: ApeTag = tag.into();

		assert_eq!(tag.track().unwrap(), track_number);
		assert!(tag.track_total().is_none());
	}

	#[test_log::test]
	fn track_total_tag_to_ape() {
		let track_total = 2;

		let mut tag = Tag::new(TagType::Ape);

		tag.push(TagItem::new(
			ItemKey::TrackTotal,
			ItemValue::Text(track_total.to_string()),
		));

		let tag: ApeTag = tag.into();

		assert_eq!(tag.track().unwrap(), DEFAULT_NUMBER_IN_PAIR);
		assert_eq!(tag.track_total().unwrap(), track_total);
	}

	#[test_log::test]
	fn track_number_and_track_total_tag_to_ape() {
		let track_number = 1;
		let track_total = 2;

		let mut tag = Tag::new(TagType::Ape);

		tag.push(TagItem::new(
			ItemKey::TrackNumber,
			ItemValue::Text(track_number.to_string()),
		));

		tag.push(TagItem::new(
			ItemKey::TrackTotal,
			ItemValue::Text(track_total.to_string()),
		));

		let tag: ApeTag = tag.into();

		assert_eq!(tag.track().unwrap(), track_number);
		assert_eq!(tag.track_total().unwrap(), track_total);
	}

	#[test_log::test]
	fn disk_number_tag_to_ape() {
		let disk_number = 1;

		let mut tag = Tag::new(TagType::Ape);

		tag.push(TagItem::new(
			ItemKey::DiscNumber,
			ItemValue::Text(disk_number.to_string()),
		));

		let tag: ApeTag = tag.into();

		assert_eq!(tag.disk().unwrap(), disk_number);
		assert!(tag.disk_total().is_none());
	}

	#[test_log::test]
	fn disk_total_tag_to_ape() {
		let disk_total = 2;

		let mut tag = Tag::new(TagType::Ape);

		tag.push(TagItem::new(
			ItemKey::DiscTotal,
			ItemValue::Text(disk_total.to_string()),
		));

		let tag: ApeTag = tag.into();

		assert_eq!(tag.disk().unwrap(), DEFAULT_NUMBER_IN_PAIR);
		assert_eq!(tag.disk_total().unwrap(), disk_total);
	}

	#[test_log::test]
	fn disk_number_and_disk_total_tag_to_ape() {
		let disk_number = 1;
		let disk_total = 2;

		let mut tag = Tag::new(TagType::Ape);

		tag.push(TagItem::new(
			ItemKey::DiscNumber,
			ItemValue::Text(disk_number.to_string()),
		));

		tag.push(TagItem::new(
			ItemKey::DiscTotal,
			ItemValue::Text(disk_total.to_string()),
		));

		let tag: ApeTag = tag.into();

		assert_eq!(tag.disk().unwrap(), disk_number);
		assert_eq!(tag.disk_total().unwrap(), disk_total);
	}

	#[test_log::test]
	fn skip_reading_cover_art() {
		let p = Picture::unchecked(std::iter::repeat_n(0, 50).collect::<Vec<u8>>())
			.pic_type(PictureType::CoverFront)
			.mime_type(MimeType::Jpeg)
			.build();

		let mut tag = Tag::new(TagType::Ape);
		tag.push_picture(p);

		tag.set_artist(String::from("Foo artist"));

		let mut writer = Vec::new();
		tag.dump_to(&mut writer, WriteOptions::new()).unwrap();

		let mut reader = Cursor::new(writer);
		let (Some(ape), _) = crate::ape::tag::read::read_ape_tag(
			&mut reader,
			false,
			ParseOptions::new().read_cover_art(false),
		)
		.unwrap() else {
			unreachable!()
		};

		assert_eq!(ape.len(), 1);
	}

	#[test_log::test]
	fn remove_disk_total_preserves_disk_number() {
		let mut ape = ApeTag::new();
		ape.set_track(2);
		ape.set_disk(3);
		ape.set_disk_total(5);

		ape.remove_disk_total();

		assert_eq!(ape.disk(), Some(3));
		assert!(ape.disk_total().is_none());
		assert_eq!(ape.track(), Some(2));
	}
}
