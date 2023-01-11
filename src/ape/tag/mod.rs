pub(crate) mod item;
pub(crate) mod read;
mod write;

use crate::ape::tag::item::{ApeItem, ApeItemRef};
use crate::error::{LoftyError, Result};
use crate::tag::item::{ItemKey, ItemValue, TagItem};
use crate::tag::{Tag, TagType};
use crate::traits::{Accessor, TagExt};

use std::borrow::Cow;
use std::convert::TryInto;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

use lofty_attr::tag;

macro_rules! impl_accessor {
	($($name:ident => $($key:literal)|+;)+) => {
		paste::paste! {
			$(
				fn $name(&self) -> Option<Cow<'_, str>> {
					$(
						if let Some(i) = self.get_key($key) {
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
						self.remove_key($key);
					)+
				}
			)+
		}
	}
}

/// ## Item storage
///
/// `APE` isn't a very strict format. An [`ApeItem`] only restricted by its name, meaning it can use
/// a normal [`ItemValue`](crate::ItemValue) unlike other formats.
///
/// Pictures are stored as [`ItemValue::Binary`](crate::ItemValue::Binary), and can be converted with
/// [`Picture::from_ape_bytes`](crate::Picture::from_ape_bytes). For the appropriate item keys, see
/// [`APE_PICTURE_TYPES`](crate::ape::APE_PICTURE_TYPES).
///
/// ## Conversions
///
/// ### From `Tag`
///
/// When converting pictures, any of type [`PictureType::Undefined`](crate::PictureType::Undefined) will be discarded.
/// For items, see [`ApeItem::new`].
#[derive(Default, Debug, PartialEq, Eq, Clone)]
#[tag(description = "An `APE` tag", supported_formats(APE, MPEG, WavPack))]
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
		self.items.retain(|i| !i.key().eq_ignore_ascii_case(key));
	}

	fn split_num_pair(&self, key: &str) -> (Option<u32>, Option<u32>) {
		if let Some(ApeItem {
			value: ItemValue::Text(ref text),
			..
		}) = self.get_key(key)
		{
			let mut split = text.split('/').flat_map(str::parse::<u32>);
			return (split.next(), split.next());
		}

		(None, None)
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
		self.insert(ApeItem::text("Track", value.to_string()))
	}

	fn remove_track(&mut self) {
		self.remove_key("Track");
	}

	fn track_total(&self) -> Option<u32> {
		self.split_num_pair("Track").1
	}

	fn set_track_total(&mut self, value: u32) {
		let current_track = self.split_num_pair("Track").0.unwrap_or(1);

		self.insert(ApeItem::text("Track", format!("{current_track}/{value}")));
	}

	fn remove_track_total(&mut self) {
		let existing_track_number = self.track();
		self.remove_key("Track");

		if let Some(track) = existing_track_number {
			self.insert(ApeItem::text("Track", track.to_string()));
		}
	}

	fn disk(&self) -> Option<u32> {
		self.split_num_pair("Disc").0
	}

	fn set_disk(&mut self, value: u32) {
		self.insert(ApeItem::text("Disc", value.to_string()));
	}

	fn remove_disk(&mut self) {
		self.remove_key("Disc");
	}

	fn disk_total(&self) -> Option<u32> {
		self.split_num_pair("Disc").1
	}

	fn set_disk_total(&mut self, value: u32) {
		let current_disk = self.split_num_pair("Disc").0.unwrap_or(1);

		self.insert(ApeItem::text("Disc", format!("{current_disk}/{value}")));
	}

	fn remove_disk_total(&mut self) {
		let existing_track_number = self.track();
		self.remove_key("Disc");

		if let Some(track) = existing_track_number {
			self.insert(ApeItem::text("Disc", track.to_string()));
		}
	}

	fn year(&self) -> Option<u32> {
		if let Some(ApeItem {
			value: ItemValue::Text(ref text),
			..
		}) = self.get_key("Year")
		{
			return text.chars().take(4).collect::<String>().parse::<u32>().ok();
		}

		None
	}

	fn set_year(&mut self, value: u32) {
		self.insert(ApeItem::text("Year", value.to_string()));
	}

	fn remove_year(&mut self) {
		self.remove_key("Year");
	}
}

impl TagExt for ApeTag {
	type Err = LoftyError;
	type RefKey<'a> = &'a str;

	fn len(&self) -> usize {
		self.items.len()
	}

	fn contains<'a>(&'a self, key: Self::RefKey<'a>) -> bool {
		self.items.iter().any(|i| i.key().eq_ignore_ascii_case(key))
	}

	fn is_empty(&self) -> bool {
		self.items.is_empty()
	}

	/// Writes the tag to a path
	///
	/// # Errors
	///
	/// * `path` does not exist
	/// * See [`ApeTag::save_to`]
	fn save_to_path<P: AsRef<Path>>(&self, path: P) -> std::result::Result<(), Self::Err> {
		self.save_to(&mut OpenOptions::new().read(true).write(true).open(path)?)
	}

	/// Write an `APE` tag to a file
	///
	/// # Errors
	///
	/// * Attempting to write the tag to a format that does not support it
	/// * An existing tag has an invalid size
	fn save_to(&self, file: &mut File) -> std::result::Result<(), Self::Err> {
		ApeTagRef {
			read_only: self.read_only,
			items: self.items.iter().map(Into::into),
		}
		.write_to(file)
	}

	/// Dumps the tag to a writer
	///
	/// # Errors
	///
	/// * [`std::io::Error`]
	fn dump_to<W: Write>(&self, writer: &mut W) -> std::result::Result<(), Self::Err> {
		ApeTagRef {
			read_only: self.read_only,
			items: self.items.iter().map(Into::into),
		}
		.dump_to(writer)
	}

	fn remove_from_path<P: AsRef<Path>>(&self, path: P) -> std::result::Result<(), Self::Err> {
		TagType::APE.remove_from_path(path)
	}

	fn remove_from(&self, file: &mut File) -> std::result::Result<(), Self::Err> {
		TagType::APE.remove_from(file)
	}

	fn clear(&mut self) {
		self.items.clear();
	}
}

impl From<ApeTag> for Tag {
	fn from(input: ApeTag) -> Self {
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

		let mut tag = Tag::new(TagType::APE);

		for item in input.items {
			let item_key = ItemKey::from_key(TagType::APE, item.key());

			// The text pairs need some special treatment
			match (item_key, item.value()) {
				(ItemKey::TrackNumber | ItemKey::TrackTotal, ItemValue::Text(val))
					if split_pair(val, &mut tag, ItemKey::TrackNumber, ItemKey::TrackTotal)
						.is_some() =>
				{
					continue
				},
				(ItemKey::DiscNumber | ItemKey::DiscTotal, ItemValue::Text(val))
					if split_pair(val, &mut tag, ItemKey::DiscNumber, ItemKey::DiscTotal)
						.is_some() =>
				{
					continue
				},
				(k, _) => tag.items.push(TagItem::new(k, item.value)),
			}
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
	pub(crate) fn write_to(&mut self, file: &mut File) -> Result<()> {
		write::write_to(file, self)
	}

	pub(crate) fn dump_to<W: Write>(&mut self, writer: &mut W) -> Result<()> {
		let temp = write::create_ape_tag(self)?;
		writer.write_all(&temp)?;

		Ok(())
	}
}

pub(crate) fn tagitems_into_ape(items: &[TagItem]) -> impl Iterator<Item = ApeItemRef<'_>> {
	items.iter().filter_map(|i| {
		i.key().map_key(TagType::APE, true).map(|key| ApeItemRef {
			read_only: false,
			key,
			value: (&i.item_value).into(),
		})
	})
}

#[cfg(test)]
mod tests {
	use crate::ape::header::read_ape_header;
	use crate::ape::{ApeItem, ApeTag};
	use crate::{ItemValue, Tag, TagExt, TagType};

	use std::io::{Cursor, Seek, SeekFrom};

	#[test]
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

		// Remove the APE preamble
		reader.seek(SeekFrom::Current(8)).unwrap();

		let header = read_ape_header(&mut reader, false).unwrap();
		let parsed_tag = crate::ape::tag::read::read_ape_tag(&mut reader, header).unwrap();

		assert_eq!(expected_tag.len(), parsed_tag.len());

		for item in &expected_tag.items {
			assert!(parsed_tag.items.contains(item));
		}
	}

	#[test]
	fn ape_re_read() {
		let tag_bytes = crate::tag::utils::test_utils::read_path("tests/tags/assets/test.apev2");
		let mut reader = Cursor::new(tag_bytes);

		// Remove the APE preamble
		reader.seek(SeekFrom::Current(8)).unwrap();

		let header = read_ape_header(&mut reader, false).unwrap();
		let parsed_tag = crate::ape::tag::read::read_ape_tag(&mut reader, header).unwrap();

		let mut writer = Vec::new();
		parsed_tag.dump_to(&mut writer).unwrap();

		let mut temp_reader = Cursor::new(writer);

		// Remove the APE preamble
		temp_reader.seek(SeekFrom::Current(8)).unwrap();

		let temp_header = read_ape_header(&mut temp_reader, false).unwrap();
		let temp_parsed_tag =
			crate::ape::tag::read::read_ape_tag(&mut temp_reader, temp_header).unwrap();

		assert_eq!(parsed_tag, temp_parsed_tag);
	}

	#[test]
	fn ape_to_tag() {
		let tag_bytes = crate::tag::utils::test_utils::read_path("tests/tags/assets/test.apev2");
		let mut reader = Cursor::new(tag_bytes);

		// Remove the APE preamble
		reader.seek(SeekFrom::Current(8)).unwrap();

		let header = read_ape_header(&mut reader, false).unwrap();
		let ape = crate::ape::tag::read::read_ape_tag(&mut reader, header).unwrap();

		let tag: Tag = ape.into();

		crate::tag::utils::test_utils::verify_tag(&tag, true, true);
	}

	#[test]
	fn tag_to_ape() {
		fn verify_key(tag: &ApeTag, key: &str, expected_val: &str) {
			assert_eq!(
				tag.get_key(key).map(ApeItem::value),
				Some(&ItemValue::Text(String::from(expected_val)))
			);
		}

		let tag = crate::tag::utils::test_utils::create_tag(TagType::APE);

		let ape_tag: ApeTag = tag.into();

		verify_key(&ape_tag, "Title", "Foo title");
		verify_key(&ape_tag, "Artist", "Bar artist");
		verify_key(&ape_tag, "Album", "Baz album");
		verify_key(&ape_tag, "Comment", "Qux comment");
		verify_key(&ape_tag, "Track", "1");
		verify_key(&ape_tag, "Genre", "Classical");
	}
}
