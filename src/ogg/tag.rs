use crate::error::{LoftyError, Result};
use crate::file::FileType;
use crate::macros::err;
use crate::ogg::picture_storage::OggPictureStorage;
use crate::ogg::write::OGGFormat;
use crate::picture::{Picture, PictureInformation};
use crate::probe::Probe;
use crate::tag::item::{ItemKey, ItemValue, TagItem};
use crate::tag::{try_parse_year, Tag, TagType};
use crate::traits::{Accessor, MergeTag, SplitTag, TagExt};

use std::borrow::Cow;
use std::fs::File;
use std::io::Write;
use std::ops::Deref;
use std::path::Path;

use lofty_attr::tag;

macro_rules! impl_accessor {
	($($name:ident => $key:literal;)+) => {
		paste::paste! {
			$(
				fn $name(&self) -> Option<Cow<'_, str>> {
					self.get($key).map(Cow::Borrowed)
				}

				fn [<set_ $name>](&mut self, value: String) {
					self.insert(String::from($key), value)
				}

				fn [<remove_ $name>](&mut self) {
					let _ = self.remove($key);
				}
			)+
		}
	}
}

#[derive(Default, PartialEq, Eq, Debug, Clone)]
#[tag(
	description = "Vorbis comments",
	supported_formats(Flac, Opus, Speex, Vorbis)
)]
pub struct VorbisComments {
	/// An identifier for the encoding software
	pub(crate) vendor: String,
	/// A collection of key-value pairs
	pub(crate) items: Vec<(String, String)>,
	/// A collection of all pictures
	pub(crate) pictures: Vec<(Picture, PictureInformation)>,
}

impl VorbisComments {
	/// Create a new empty `VorbisComments`
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ogg::VorbisComments;
	/// use lofty::TagExt;
	///
	/// let vorbis_comments_tag = VorbisComments::new();
	/// assert!(vorbis_comments_tag.is_empty());
	/// ```
	pub fn new() -> Self {
		Self::default()
	}

	/// Returns the vendor string
	///
	/// ```rust
	/// use lofty::ogg::VorbisComments;
	///
	/// let mut vorbis_comments = VorbisComments::default();
	/// assert!(vorbis_comments.vendor().is_empty());
	///
	/// vorbis_comments.set_vendor(String::from("FooBar"));
	/// assert_eq!(vorbis_comments.vendor(), "FooBar");
	/// ```
	pub fn vendor(&self) -> &str {
		&self.vendor
	}

	/// Sets the vendor string
	///
	/// ```rust
	/// use lofty::ogg::VorbisComments;
	///
	/// let mut vorbis_comments = VorbisComments::default();
	///
	/// vorbis_comments.set_vendor(String::from("FooBar"));
	/// assert_eq!(vorbis_comments.vendor(), "FooBar");
	/// ```
	pub fn set_vendor(&mut self, vendor: String) {
		self.vendor = vendor
	}

	/// Get all items
	///
	/// Returns an [`Iterator`] over the stored key/value pairs.
	///
	/// ```rust
	/// use lofty::ogg::VorbisComments;
	///
	/// let mut vorbis_comments = VorbisComments::default();
	///
	/// vorbis_comments.push(String::from("ARTIST"), String::from("Foo artist"));
	/// vorbis_comments.push(String::from("TITLE"), String::from("Bar title"));
	///
	/// let mut items = vorbis_comments.items();
	///
	/// assert_eq!(items.next(), Some(("ARTIST", "Foo artist")));
	/// assert_eq!(items.next(), Some(("TITLE", "Bar title")));
	/// ```
	pub fn items(&self) -> impl ExactSizeIterator<Item = (&str, &str)> + Clone {
		self.items.iter().map(|(k, v)| (k.as_str(), v.as_str()))
	}

	/// Consume all items
	///
	/// Returns an [`Iterator`] with the stored key/value pairs.
	///
	/// ```rust
	/// use lofty::ogg::VorbisComments;
	/// use lofty::TagExt;
	///
	/// let mut vorbis_comments = VorbisComments::default();
	///
	/// vorbis_comments.push(String::from("ARTIST"), String::from("Foo artist"));
	/// vorbis_comments.push(String::from("TITLE"), String::from("Bar title"));
	///
	/// for (key, value) in vorbis_comments.take_items() {
	/// 	println!("We took field: {key}={value}");
	/// }
	///
	/// // We've taken all the items
	/// assert!(vorbis_comments.is_empty());
	/// ```
	pub fn take_items(&mut self) -> impl ExactSizeIterator<Item = (String, String)> {
		let items = std::mem::take(&mut self.items);
		items.into_iter()
	}

	/// Gets the first item with `key`
	///
	/// NOTE: There can be multiple items with the same key, this grabs whichever happens to be the first
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ogg::VorbisComments;
	///
	/// let mut vorbis_comments = VorbisComments::default();
	///
	/// // Vorbis comments allows multiple fields with the same key, such as artist
	/// vorbis_comments.push(String::from("ARTIST"), String::from("Foo artist"));
	/// vorbis_comments.push(String::from("ARTIST"), String::from("Bar artist"));
	/// vorbis_comments.push(String::from("ARTIST"), String::from("Baz artist"));
	///
	/// let first_artist = vorbis_comments.get("ARTIST").unwrap();
	/// assert_eq!(first_artist, "Foo artist");
	/// ```
	pub fn get(&self, key: &str) -> Option<&str> {
		if !verify_key(key) {
			return None;
		}

		self.items
			.iter()
			.find(|(k, _)| k.eq_ignore_ascii_case(key))
			.map(|(_, v)| v.as_str())
	}

	/// Gets all items with the key
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ogg::VorbisComments;
	///
	/// let mut vorbis_comments = VorbisComments::default();
	///
	/// // Vorbis comments allows multiple fields with the same key, such as artist
	/// vorbis_comments.push(String::from("ARTIST"), String::from("Foo artist"));
	/// vorbis_comments.push(String::from("ARTIST"), String::from("Bar artist"));
	/// vorbis_comments.push(String::from("ARTIST"), String::from("Baz artist"));
	///
	/// let all_artists = vorbis_comments.get_all("ARTIST").collect::<Vec<&str>>();
	/// assert_eq!(all_artists, vec!["Foo artist", "Bar artist", "Baz artist"]);
	/// ```
	pub fn get_all<'a>(&'a self, key: &'a str) -> impl Iterator<Item = &'a str> + Clone + '_ {
		self.items
			.iter()
			.filter_map(move |(k, v)| (k.eq_ignore_ascii_case(key)).then_some(v.as_str()))
	}

	/// Inserts an item
	///
	/// This is the same as [`VorbisComments::push`], except it will remove any items with the same key.
	///
	/// NOTE: This will do nothing if the key is invalid. This specification is available [here](https://xiph.org/vorbis/doc/v-comment.html#vectorformat).
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ogg::VorbisComments;
	///
	/// let mut tag = VorbisComments::default();
	/// tag.insert(String::from("TITLE"), String::from("Title 1"));
	/// tag.insert(String::from("TITLE"), String::from("Title 2"));
	///
	/// // We only retain the last title inserted
	/// let mut titles = tag.get_all("TITLE");
	/// assert_eq!(titles.next(), Some("Title 2"));
	/// assert_eq!(titles.next(), None);
	/// ```
	pub fn insert(&mut self, key: String, value: String) {
		if !verify_key(&key) {
			return;
		}

		self.items.retain(|(k, _)| !k.eq_ignore_ascii_case(&key));
		self.items.push((key, value))
	}

	/// Appends an item
	///
	/// NOTE: This will do nothing if the key is invalid. This specification is available [here](https://xiph.org/vorbis/doc/v-comment.html#vectorformat).
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ogg::VorbisComments;
	///
	/// let mut tag = VorbisComments::default();
	/// tag.push(String::from("TITLE"), String::from("Title 1"));
	/// tag.push(String::from("TITLE"), String::from("Title 2"));
	///
	/// // We retain both titles
	/// let mut titles = tag.get_all("TITLE");
	/// assert_eq!(titles.next(), Some("Title 1"));
	/// assert_eq!(titles.next(), Some("Title 2"));
	/// ```
	pub fn push(&mut self, key: String, value: String) {
		if !verify_key(&key) {
			return;
		}

		self.items.push((key, value))
	}

	/// Removes all items with a key, returning an iterator
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ogg::VorbisComments;
	///
	/// let mut tag = VorbisComments::default();
	/// tag.push(String::from("TITLE"), String::from("Title 1"));
	/// tag.push(String::from("TITLE"), String::from("Title 2"));
	///
	/// // Remove both titles
	/// for title in tag.remove("TITLE") {
	/// 	println!("We removed the title: {title}");
	/// }
	/// ```
	pub fn remove(&mut self, key: &str) -> impl Iterator<Item = String> + '_ {
		// TODO: drain_filter
		let mut split_idx = 0_usize;

		for read_idx in 0..self.items.len() {
			if self.items[read_idx].0.eq_ignore_ascii_case(key) {
				self.items.swap(split_idx, read_idx);
				split_idx += 1;
			}
		}

		self.items.drain(..split_idx).map(|(_, v)| v)
	}
}

// A case-insensitive field name that may consist of ASCII 0x20 through 0x7D, 0x3D ('=') excluded.
// ASCII 0x41 through 0x5A inclusive (A-Z) is to be considered equivalent to ASCII 0x61 through 0x7A inclusive (a-z).
fn verify_key(key: &str) -> bool {
	if key.is_empty() {
		return false;
	}

	key.bytes()
		.all(|byte| (0x20..=0x7D).contains(&byte) && byte != 0x3D)
}

impl OggPictureStorage for VorbisComments {
	fn pictures(&self) -> &[(Picture, PictureInformation)] {
		&self.pictures
	}
}

impl Accessor for VorbisComments {
	impl_accessor!(
		artist  => "ARTIST";
		title   => "TITLE";
		album   => "ALBUM";
		genre   => "GENRE";
		comment => "COMMENT";
	);

	fn track(&self) -> Option<u32> {
		if let Some(item) = self
			.get("TRACKNUMBER")
			.map_or_else(|| self.get("TRACKNUM"), Some)
		{
			return item.parse::<u32>().ok();
		}

		None
	}

	fn set_track(&mut self, value: u32) {
		self.remove_track();
		self.insert(String::from("TRACKNUMBER"), value.to_string());
	}

	fn remove_track(&mut self) {
		let _ = self.remove("TRACKNUMBER");
		let _ = self.remove("TRACKNUM");
	}

	fn track_total(&self) -> Option<u32> {
		if let Some(item) = self
			.get("TRACKTOTAL")
			.map_or_else(|| self.get("TOTALTRACKS"), Some)
		{
			return item.parse::<u32>().ok();
		}

		None
	}

	fn set_track_total(&mut self, value: u32) {
		self.insert(String::from("TRACKTOTAL"), value.to_string());
		let _ = self.remove("TOTALTRACKS");
	}

	fn remove_track_total(&mut self) {
		let _ = self.remove("TRACKTOTAL");
		let _ = self.remove("TOTALTRACKS");
	}

	fn disk(&self) -> Option<u32> {
		if let Some(item) = self.get("DISCNUMBER") {
			return item.parse::<u32>().ok();
		}

		None
	}

	fn set_disk(&mut self, value: u32) {
		self.insert(String::from("DISCNUMBER"), value.to_string());
	}

	fn remove_disk(&mut self) {
		let _ = self.remove("DISCNUMBER");
	}

	fn disk_total(&self) -> Option<u32> {
		if let Some(item) = self
			.get("DISCTOTAL")
			.map_or_else(|| self.get("TOTALDISCS"), Some)
		{
			return item.parse::<u32>().ok();
		}

		None
	}

	fn set_disk_total(&mut self, value: u32) {
		self.insert(String::from("DISCTOTAL"), value.to_string());
		let _ = self.remove("TOTALDISCS");
	}

	fn remove_disk_total(&mut self) {
		let _ = self.remove("DISCTOTAL");
		let _ = self.remove("TOTALDISCS");
	}

	fn year(&self) -> Option<u32> {
		if let Some(item) = self.get("YEAR").map_or_else(|| self.get("DATE"), Some) {
			return try_parse_year(item);
		}

		None
	}

	fn set_year(&mut self, value: u32) {
		// DATE is the preferred way of storing the year, but it is still possible we will
		// encounter YEAR
		self.insert(String::from("DATE"), value.to_string());
		let _ = self.remove("YEAR");
	}

	fn remove_year(&mut self) {
		// DATE is not valid without a year, so we can remove them as well
		let _ = self.remove("DATE");
		let _ = self.remove("YEAR");
	}
}

impl TagExt for VorbisComments {
	type Err = LoftyError;
	type RefKey<'a> = &'a str;

	fn len(&self) -> usize {
		self.items.len() + self.pictures.len()
	}

	fn contains<'a>(&'a self, key: Self::RefKey<'a>) -> bool {
		self.items
			.iter()
			.any(|(item_key, _)| item_key.eq_ignore_ascii_case(key))
	}

	fn is_empty(&self) -> bool {
		self.items.is_empty() && self.pictures.is_empty()
	}

	/// Writes the tag to a file
	///
	/// # Errors
	///
	/// * Attempting to write the tag to a format that does not support it
	/// * The file does not contain valid packets
	/// * [`PictureInformation::from_picture`]
	/// * [`std::io::Error`]
	fn save_to(&self, file: &mut File) -> std::result::Result<(), Self::Err> {
		VorbisCommentsRef {
			vendor: self.vendor.as_str(),
			items: self.items.iter().map(|(k, v)| (k.as_str(), v.as_str())),
			pictures: self.pictures.iter().map(|(p, i)| (p, *i)),
		}
		.write_to(file)
	}

	/// Dumps the tag to a writer
	///
	/// This does not include a vendor string, and will thus
	/// not create a usable file.
	///
	/// # Errors
	///
	/// * [`PictureInformation::from_picture`]
	/// * [`std::io::Error`]
	fn dump_to<W: Write>(&self, writer: &mut W) -> std::result::Result<(), Self::Err> {
		VorbisCommentsRef {
			vendor: self.vendor.as_str(),
			items: self.items.iter().map(|(k, v)| (k.as_str(), v.as_str())),
			pictures: self.pictures.iter().map(|(p, i)| (p, *i)),
		}
		.dump_to(writer)
	}

	fn remove_from_path<P: AsRef<Path>>(&self, path: P) -> std::result::Result<(), Self::Err> {
		TagType::VorbisComments.remove_from_path(path)
	}

	fn remove_from(&self, file: &mut File) -> std::result::Result<(), Self::Err> {
		TagType::VorbisComments.remove_from(file)
	}

	fn clear(&mut self) {
		self.items.clear();
		self.pictures.clear();
	}
}

#[derive(Debug, Clone, Default)]
pub struct SplitTagRemainder(VorbisComments);

impl From<SplitTagRemainder> for VorbisComments {
	fn from(from: SplitTagRemainder) -> Self {
		from.0
	}
}

impl Deref for SplitTagRemainder {
	type Target = VorbisComments;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl SplitTag for VorbisComments {
	type Remainder = SplitTagRemainder;

	fn split_tag(mut self) -> (Self::Remainder, Tag) {
		let mut tag = Tag::new(TagType::VorbisComments);

		for (k, v) in std::mem::take(&mut self.items) {
			tag.items.push(TagItem::new(
				ItemKey::from_key(TagType::VorbisComments, &k),
				ItemValue::Text(v),
			));
		}

		// We need to preserve the vendor string
		if !tag
			.items
			.iter()
			.any(|i| i.key() == &ItemKey::EncoderSoftware)
		{
			tag.items.push(TagItem::new(
				ItemKey::EncoderSoftware,
				// Preserve the original vendor by cloning
				ItemValue::Text(self.vendor.clone()),
			));
		}

		for (pic, _info) in std::mem::take(&mut self.pictures) {
			tag.push_picture(pic)
		}

		(SplitTagRemainder(self), tag)
	}
}

impl MergeTag for SplitTagRemainder {
	type Merged = VorbisComments;

	fn merge_tag(self, mut tag: Tag) -> Self::Merged {
		let Self(mut merged) = self;

		if let Some(TagItem {
			item_value: ItemValue::Text(val),
			..
		}) = tag.take(&ItemKey::EncoderSoftware).next()
		{
			merged.vendor = val;
		}

		for item in tag.items {
			let item_key = item.item_key;
			let item_value = item.item_value;

			// Discard binary items, as they are not allowed in Vorbis comments
			let val = match item_value {
				ItemValue::Text(text) | ItemValue::Locator(text) => text,
				_ => continue,
			};

			let key = match item_key.map_key(TagType::VorbisComments, true) {
				None => continue,
				Some(k) => k,
			};

			merged.items.push((key.to_string(), val));
		}

		for picture in tag.pictures {
			if let Ok(information) = PictureInformation::from_picture(&picture) {
				merged.pictures.push((picture, information))
			}
		}

		merged
	}
}

impl From<VorbisComments> for Tag {
	fn from(input: VorbisComments) -> Self {
		input.split_tag().1
	}
}

impl From<Tag> for VorbisComments {
	fn from(input: Tag) -> Self {
		SplitTagRemainder::default().merge_tag(input)
	}
}

pub(crate) struct VorbisCommentsRef<'a, II, IP>
where
	II: Iterator<Item = (&'a str, &'a str)>,
	IP: Iterator<Item = (&'a Picture, PictureInformation)>,
{
	pub vendor: &'a str,
	pub items: II,
	pub pictures: IP,
}

impl<'a, II, IP> VorbisCommentsRef<'a, II, IP>
where
	II: Iterator<Item = (&'a str, &'a str)>,
	IP: Iterator<Item = (&'a Picture, PictureInformation)>,
{
	#[allow(clippy::shadow_unrelated)]
	pub(crate) fn write_to(&mut self, file: &mut File) -> Result<()> {
		let probe = Probe::new(file).guess_file_type()?;
		let f_ty = probe.file_type();

		let file = probe.into_inner();

		let file_type = match f_ty {
			Some(ft) if VorbisComments::SUPPORTED_FORMATS.contains(&ft) => ft,
			_ => err!(UnsupportedTag),
		};

		// FLAC has its own special writing needs :)
		if file_type == FileType::Flac {
			return crate::flac::write::write_to_inner(file, self);
		}

		let (format, header_packet_count) = OGGFormat::from_filetype(file_type);

		super::write::write(file, self, format, header_packet_count)
	}

	pub(crate) fn dump_to<W: Write>(&mut self, writer: &mut W) -> Result<()> {
		let metadata_packet =
			super::write::create_metadata_packet(self, &[], self.vendor.as_bytes(), false)?;
		writer.write_all(&metadata_packet)?;
		Ok(())
	}
}

pub(crate) fn create_vorbis_comments_ref(
	tag: &Tag,
) -> (
	&str,
	impl Iterator<Item = (&str, &str)>,
	impl Iterator<Item = (&Picture, PictureInformation)>,
) {
	let vendor = tag.get_string(&ItemKey::EncoderSoftware).unwrap_or("");

	let items = tag.items.iter().filter_map(|i| match i.value() {
		ItemValue::Text(val) | ItemValue::Locator(val) => i
			.key()
			.map_key(TagType::VorbisComments, true)
			.map(|key| (key, val.as_str())),
		_ => None,
	});

	let pictures = tag
		.pictures
		.iter()
		.map(|p| (p, PictureInformation::from_picture(p).unwrap_or_default()));
	(vendor, items, pictures)
}

#[cfg(test)]
mod tests {
	use crate::ogg::{OggPictureStorage, VorbisComments};
	use crate::{
		ItemKey, ItemValue, MergeTag as _, ParsingMode, SplitTag as _, Tag, TagExt as _, TagItem,
		TagType,
	};

	fn read_tag(tag: &[u8]) -> VorbisComments {
		let mut reader = std::io::Cursor::new(tag);

		crate::ogg::read::read_comments(&mut reader, tag.len() as u64, ParsingMode::Strict).unwrap()
	}

	#[test]
	fn parse_vorbis_comments() {
		let mut expected_tag = VorbisComments::default();

		expected_tag.set_vendor(String::from("Lavf58.76.100"));

		expected_tag.push(String::from("ALBUM"), String::from("Baz album"));
		expected_tag.push(String::from("ARTIST"), String::from("Bar artist"));
		expected_tag.push(String::from("COMMENT"), String::from("Qux comment"));
		expected_tag.push(String::from("DATE"), String::from("1984"));
		expected_tag.push(String::from("GENRE"), String::from("Classical"));
		expected_tag.push(String::from("TITLE"), String::from("Foo title"));
		expected_tag.push(String::from("TRACKNUMBER"), String::from("1"));

		let file_cont = crate::tag::utils::test_utils::read_path("tests/tags/assets/test.vorbis");
		let parsed_tag = read_tag(&file_cont);

		assert_eq!(expected_tag, parsed_tag);
	}

	#[test]
	fn vorbis_comments_re_read() {
		let file_cont = crate::tag::utils::test_utils::read_path("tests/tags/assets/test.vorbis");
		let mut parsed_tag = read_tag(&file_cont);

		// Create a zero-size vendor for comparison
		parsed_tag.vendor = String::new();

		let mut writer = Vec::new();
		parsed_tag.dump_to(&mut writer).unwrap();

		let temp_parsed_tag = read_tag(&writer);

		assert_eq!(parsed_tag, temp_parsed_tag);
	}

	#[test]
	fn vorbis_comments_to_tag() {
		let tag_bytes = std::fs::read("tests/tags/assets/test.vorbis").unwrap();
		let vorbis_comments = read_tag(&tag_bytes);

		let tag: Tag = vorbis_comments.into();

		crate::tag::utils::test_utils::verify_tag(&tag, true, true);
	}

	#[test]
	fn tag_to_vorbis_comments() {
		let tag = crate::tag::utils::test_utils::create_tag(TagType::VorbisComments);

		let vorbis_comments: VorbisComments = tag.into();

		assert_eq!(vorbis_comments.get("TITLE"), Some("Foo title"));
		assert_eq!(vorbis_comments.get("ARTIST"), Some("Bar artist"));
		assert_eq!(vorbis_comments.get("ALBUM"), Some("Baz album"));
		assert_eq!(vorbis_comments.get("COMMENT"), Some("Qux comment"));
		assert_eq!(vorbis_comments.get("TRACKNUMBER"), Some("1"));
		assert_eq!(vorbis_comments.get("GENRE"), Some("Classical"));
	}

	#[test]
	fn multi_value_roundtrip() {
		let mut tag = Tag::new(TagType::VorbisComments);
		tag.insert_text(ItemKey::TrackArtist, "TrackArtist 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::TrackArtist,
			ItemValue::Text("TrackArtist 2".to_owned()),
		));
		tag.insert_text(ItemKey::AlbumArtist, "AlbumArtist 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::AlbumArtist,
			ItemValue::Text("AlbumArtist 2".to_owned()),
		));
		tag.insert_text(ItemKey::TrackTitle, "TrackTitle 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::TrackTitle,
			ItemValue::Text("TrackTitle 2".to_owned()),
		));
		tag.insert_text(ItemKey::AlbumTitle, "AlbumTitle 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::AlbumTitle,
			ItemValue::Text("AlbumTitle 2".to_owned()),
		));
		tag.insert_text(ItemKey::Comment, "Comment 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::Comment,
			ItemValue::Text("Comment 2".to_owned()),
		));
		tag.insert_text(ItemKey::ContentGroup, "ContentGroup 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::ContentGroup,
			ItemValue::Text("ContentGroup 2".to_owned()),
		));
		tag.insert_text(ItemKey::Genre, "Genre 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::Genre,
			ItemValue::Text("Genre 2".to_owned()),
		));
		tag.insert_text(ItemKey::Mood, "Mood 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::Mood,
			ItemValue::Text("Mood 2".to_owned()),
		));
		tag.insert_text(ItemKey::Composer, "Composer 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::Composer,
			ItemValue::Text("Composer 2".to_owned()),
		));
		tag.insert_text(ItemKey::Conductor, "Conductor 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::Conductor,
			ItemValue::Text("Conductor 2".to_owned()),
		));
		// Otherwise the following item would be inserted implicitly
		// during the conversion.
		tag.insert_text(ItemKey::EncoderSoftware, "EncoderSoftware".to_owned());
		assert_eq!(20 + 1, tag.len());

		let mut vorbis_comments1 = VorbisComments::from(tag.clone());

		let (split_remainder, split_tag) = vorbis_comments1.clone().split_tag();
		assert_eq!(0, split_remainder.len());
		assert_eq!(tag.len(), split_tag.len());

		// Merge back into Vorbis Comments for comparison
		let mut vorbis_comments2 = split_remainder.merge_tag(split_tag);
		// Soft before comparison -> unordered comparison
		vorbis_comments1
			.items
			.sort_by(|(lhs_key, lhs_val), (rhs_key, rhs_val)| {
				lhs_key.cmp(rhs_key).then_with(|| lhs_val.cmp(rhs_val))
			});
		vorbis_comments2
			.items
			.sort_by(|(lhs_key, lhs_val), (rhs_key, rhs_val)| {
				lhs_key.cmp(rhs_key).then_with(|| lhs_val.cmp(rhs_val))
			});
		assert_eq!(vorbis_comments1.items, vorbis_comments2.items);
	}

	#[test]
	fn zero_sized_vorbis_comments() {
		let tag_bytes = std::fs::read("tests/tags/assets/zero.vorbis").unwrap();
		let _ = read_tag(&tag_bytes);
	}

	#[test]
	fn issue_60() {
		let tag_bytes = std::fs::read("tests/tags/assets/issue_60.vorbis").unwrap();
		let tag = read_tag(&tag_bytes);

		assert_eq!(tag.pictures().len(), 1);
		assert!(tag.items.is_empty());
	}

	#[test]
	fn initial_key_roundtrip() {
		// Both the primary and alternate key should be mapped to the primary
		// key if stored again. Note: The outcome is undefined if both the
		// primary and alternate key would be stored redundantly in VorbisComments!
		for key in ["INITIALKEY", "KEY"] {
			let mut vorbis_comments = VorbisComments {
				items: vec![(key.to_owned(), "Cmaj".to_owned())],
				..Default::default()
			};
			let mut tag = Tag::try_from(vorbis_comments).unwrap();
			assert_eq!(Some("Cmaj"), tag.get_string(&ItemKey::InitialKey));
			tag.insert_text(ItemKey::InitialKey, "Cmin".to_owned());
			vorbis_comments = tag.into();
			assert_eq!(Some("Cmin"), vorbis_comments.get("INITIALKEY"));
		}
	}
}
