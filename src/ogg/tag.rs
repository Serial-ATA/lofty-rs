use crate::error::{LoftyError, Result};
use crate::file::FileType;
use crate::macros::err;
use crate::ogg::picture_storage::OggPictureStorage;
use crate::ogg::write::OGGFormat;
use crate::picture::{Picture, PictureInformation};
use crate::probe::Probe;
use crate::tag::item::{ItemKey, ItemValue, TagItem};
use crate::tag::{Tag, TagType};
use crate::traits::{Accessor, TagExt};

use std::borrow::Cow;
use std::fs::{File, OpenOptions};
use std::io::Write;
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
					self.insert(String::from($key), value, true)
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
	supported_formats(FLAC, Opus, Speex, Vorbis)
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
	/// Returns the vendor string
	pub fn vendor(&self) -> &str {
		&self.vendor
	}

	/// Sets the vendor string
	pub fn set_vendor(&mut self, vendor: String) {
		self.vendor = vendor
	}

	/// Visit all items
	///
	/// Returns an [`Iterator`] over the stored key/value pairs.
	pub fn items(&self) -> impl ExactSizeIterator<Item = (&str, &str)> + Clone {
		self.items.iter().map(|(k, v)| (k.as_str(), v.as_str()))
	}

	/// Consume all items
	///
	/// Returns an [`Iterator`] with the stored key/value pairs.
	pub fn take_items(&mut self) -> impl ExactSizeIterator<Item = (String, String)> {
		let items = std::mem::take(&mut self.items);
		items.into_iter()
	}

	/// Gets an item by key
	///
	/// NOTE: This is case-sensitive
	pub fn get(&self, key: &str) -> Option<&str> {
		self.items
			.iter()
			.find(|(k, _)| k == key)
			.map(|(_, v)| v.as_str())
	}

	/// Gets all items with the key
	///
	/// NOTE: This is case-sensitive
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ogg::VorbisComments;
	///
	/// let mut vorbis_comments = VorbisComments::default();
	///
	/// // Vorbis comments allows multiple fields with the same key, such as artist
	/// vorbis_comments.insert(String::from("ARTIST"), String::from("Foo artist"), false);
	/// vorbis_comments.insert(String::from("ARTIST"), String::from("Bar artist"), false);
	/// vorbis_comments.insert(String::from("ARTIST"), String::from("Baz artist"), false);
	///
	/// let all_artists = vorbis_comments.get_all("ARTIST").collect::<Vec<&str>>();
	/// assert_eq!(all_artists, vec!["Foo artist", "Bar artist", "Baz artist"]);
	/// ```
	pub fn get_all<'a>(&'a self, key: &'a str) -> impl Iterator<Item = &'a str> + Clone + '_ {
		self.items
			.iter()
			.filter_map(move |(k, v)| (k == key).then_some(v.as_str()))
	}

	/// Inserts an item
	///
	/// If `replace_all` is true, it will remove all items with the key before insertion
	pub fn insert(&mut self, key: String, value: String, replace_all: bool) {
		if replace_all {
			self.items.retain(|(k, _)| k != &key);
		}

		self.items.push((key, value))
	}

	/// Removes all items with a key, returning an iterator
	///
	/// NOTE: This is case-sensitive
	pub fn remove(&mut self, key: &str) -> impl Iterator<Item = String> + '_ {
		// TODO: drain_filter
		let mut split_idx = 0_usize;

		for read_idx in 0..self.items.len() {
			if self.items[read_idx].0 == key {
				self.items.swap(split_idx, read_idx);
				split_idx += 1;
			}
		}

		self.items.drain(..split_idx).map(|(_, v)| v)
	}
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
		if let Some(item) = self.get("TRACKNUMBER") {
			return item.parse::<u32>().ok();
		}

		None
	}

	fn set_track(&mut self, value: u32) {
		self.insert(String::from("TRACKNUMBER"), value.to_string(), true);
	}

	fn remove_track(&mut self) {
		let _ = self.remove("TRACKNUMBER");
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
		self.insert(String::from("TRACKTOTAL"), value.to_string(), true);
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
		self.insert(String::from("DISCNUMBER"), value.to_string(), true);
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
		self.insert(String::from("DISCTOTAL"), value.to_string(), true);
		let _ = self.remove("TOTALDISCS");
	}

	fn remove_disk_total(&mut self) {
		let _ = self.remove("DISCTOTAL");
		let _ = self.remove("TOTALDISCS");
	}

	fn year(&self) -> Option<u32> {
		if let Some(item) = self.get("YEAR").map_or_else(|| self.get("DATE"), Some) {
			return item.chars().take(4).collect::<String>().parse::<u32>().ok();
		}

		None
	}

	fn set_year(&mut self, value: u32) {
		// DATE is the preferred way of storing the year, but it is still possible we will
		// encounter YEAR
		self.insert(String::from("DATE"), value.to_string(), true);
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

	/// Writes the tag to a path
	///
	/// # Errors
	///
	/// * `path` does not exist
	/// * See [`VorbisComments::save_to`]
	fn save_to_path<P: AsRef<Path>>(&self, path: P) -> std::result::Result<(), Self::Err> {
		self.save_to(&mut OpenOptions::new().read(true).write(true).open(path)?)
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

impl From<VorbisComments> for Tag {
	fn from(input: VorbisComments) -> Self {
		let mut tag = Tag::new(TagType::VorbisComments);

		for (k, v) in input.items {
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
				ItemValue::Text(input.vendor),
			));
		}

		for (pic, _info) in input.pictures {
			tag.push_picture(pic)
		}

		tag
	}
}

impl From<Tag> for VorbisComments {
	fn from(mut input: Tag) -> Self {
		let mut vorbis_comments = Self::default();

		if let Some(TagItem {
			item_value: ItemValue::Text(val),
			..
		}) = input.take(&ItemKey::EncoderSoftware).next()
		{
			vorbis_comments.vendor = val;
		}

		for item in input.items {
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

			vorbis_comments.items.push((key.to_string(), val));
		}

		for picture in input.pictures {
			if let Ok(information) = PictureInformation::from_picture(&picture) {
				vorbis_comments.pictures.push((picture, information))
			}
		}

		vorbis_comments
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

		let ogg_format = match file_type {
			FileType::Opus => OGGFormat::Opus,
			FileType::Vorbis => OGGFormat::Vorbis,
			FileType::Speex => OGGFormat::Speex,
			// FLAC has its own special writing needs :)
			FileType::FLAC => {
				return crate::flac::write::write_to_inner(file, self);
			},
			_ => unreachable!("You forgot to add support for FileType::{:?}!", file_type),
		};

		super::write::write(file, self, ogg_format)
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
	use crate::{Tag, TagExt, TagType};

	fn read_tag(tag: &[u8]) -> VorbisComments {
		let mut reader = std::io::Cursor::new(tag);
		let mut parsed_tag = VorbisComments::default();

		crate::ogg::read::read_comments(&mut reader, tag.len() as u64, &mut parsed_tag).unwrap();
		parsed_tag
	}

	#[test]
	fn parse_vorbis_comments() {
		let mut expected_tag = VorbisComments::default();

		expected_tag.set_vendor(String::from("Lavf58.76.100"));

		expected_tag.insert(String::from("ALBUM"), String::from("Baz album"), false);
		expected_tag.insert(String::from("ARTIST"), String::from("Bar artist"), false);
		expected_tag.insert(String::from("COMMENT"), String::from("Qux comment"), false);
		expected_tag.insert(String::from("DATE"), String::from("1984"), false);
		expected_tag.insert(String::from("GENRE"), String::from("Classical"), false);
		expected_tag.insert(String::from("TITLE"), String::from("Foo title"), false);
		expected_tag.insert(String::from("TRACKNUMBER"), String::from("1"), false);

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
}
