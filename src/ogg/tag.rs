use crate::error::{LoftyError, Result};
use crate::file::FileType;
use crate::macros::err;
use crate::ogg::write::OGGFormat;
use crate::picture::{Picture, PictureInformation, PictureType};
use crate::probe::Probe;
use crate::tag::item::{ItemKey, ItemValue, TagItem};
use crate::tag::{Tag, TagType};
use crate::traits::{Accessor, TagExt};

use std::fs::{File, OpenOptions};
use std::io::{Cursor, Write};
use std::path::Path;

use lofty_attr::tag;

macro_rules! impl_accessor {
	($($name:ident => $key:literal;)+) => {
		paste::paste! {
			$(
				fn $name(&self) -> Option<&str> {
					self.get($key)
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

	/// Returns the tag's items in (key, value) pairs
	pub fn items(&self) -> &[(String, String)] {
		&self.items
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
	pub fn get_all<'a>(&'a self, key: &'a str) -> impl Iterator<Item = &'a str> + '_ {
		self.items
			.iter()
			.filter_map(move |(k, v)| (k == key).then(|| v.as_str()))
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

	/// Inserts a [`Picture`]
	///
	/// NOTES:
	///
	/// * If `information` is `None`, the [`PictureInformation`] will be inferred using [`PictureInformation::from_picture`].
	/// * According to spec, there can only be one picture of type [`PictureType::Icon`] and [`PictureType::OtherIcon`].
	///   When attempting to insert these types, if another is found it will be removed and returned.
	///
	/// # Errors
	///
	/// * See [`PictureInformation::from_picture`]
	pub fn insert_picture(
		&mut self,
		picture: Picture,
		information: Option<PictureInformation>,
	) -> Result<Option<(Picture, PictureInformation)>> {
		let ret = match picture.pic_type {
			PictureType::Icon | PictureType::OtherIcon => self
				.pictures
				.iter()
				.position(|(p, _)| p.pic_type == picture.pic_type)
				.map(|pos| self.pictures.remove(pos)),
			_ => None,
		};

		let info = match information {
			Some(pic_info) => pic_info,
			None => PictureInformation::from_picture(&picture)?,
		};

		self.pictures.push((picture, info));

		Ok(ret)
	}

	/// Removes a certain [`PictureType`]
	pub fn remove_picture_type(&mut self, picture_type: PictureType) {
		self.pictures.retain(|(p, _)| p.pic_type != picture_type)
	}

	/// Returns the stored [`Picture`]s as a slice
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ogg::VorbisComments;
	///
	/// let mut tag = VorbisComments::default();
	///
	/// assert!(tag.pictures().is_empty());
	/// ```
	pub fn pictures(&self) -> &[(Picture, PictureInformation)] {
		&self.pictures
	}

	/// Replaces the picture at the given `index`
	///
	/// NOTE: If `index` is out of bounds, the `picture` will be appended
	/// to the list.
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ogg::VorbisComments;
	/// # use lofty::{Picture, PictureInformation, PictureType, MimeType};
	///
	/// # fn main() -> lofty::Result<()> {
	/// # let front_cover = Picture::new_unchecked(PictureType::CoverFront, MimeType::Png, None, Vec::new());
	/// # let front_cover_info = PictureInformation::default();
	/// # let back_cover = Picture::new_unchecked(PictureType::CoverBack, MimeType::Png, None, Vec::new());
	/// # let back_cover_info = PictureInformation::default();
	/// # let another_picture = Picture::new_unchecked(PictureType::Band, MimeType::Png, None, Vec::new());
	/// let mut tag = VorbisComments::default();
	///
	/// // Add a front cover
	/// tag.insert_picture(front_cover, Some(front_cover_info))?;
	///
	/// assert_eq!(tag.pictures().len(), 1);
	/// assert_eq!(tag.pictures()[0].0.pic_type(), PictureType::CoverFront);
	///
	/// // Replace the front cover with a back cover
	/// tag.set_picture(0, back_cover, back_cover_info);
	///
	/// assert_eq!(tag.pictures().len(), 1);
	/// assert_eq!(tag.pictures()[0].0.pic_type(), PictureType::CoverBack);
	///
	/// // Use an out of bounds index
	/// tag.set_picture(100, another_picture, PictureInformation::default());
	///
	/// assert_eq!(tag.pictures().len(), 2);
	/// # Ok(()) }
	/// ```
	#[allow(clippy::missing_panics_doc)]
	pub fn set_picture(&mut self, index: usize, picture: Picture, info: PictureInformation) {
		if index >= self.pictures.len() {
			// Safe to unwrap, since `info` is guaranteed to exist
			self.insert_picture(picture, Some(info)).unwrap();
		} else {
			self.pictures[index] = (picture, info);
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
	/// use lofty::ogg::VorbisComments;
	/// # use lofty::{Picture, PictureType, MimeType, PictureInformation};
	///
	/// # fn main() -> lofty::Result<()> {
	/// # let front_cover = Picture::new_unchecked(PictureType::CoverFront, MimeType::Png, None, Vec::new());
	/// # let front_cover_info = PictureInformation::default();
	/// let mut tag = VorbisComments::default();
	///
	/// // Add a front cover
	/// tag.insert_picture(front_cover, Some(front_cover_info))?;
	///
	/// assert_eq!(tag.pictures().len(), 1);
	///
	/// tag.remove_picture(0);
	///
	/// assert_eq!(tag.pictures().len(), 0);
	/// # Ok(()) }
	/// ```
	pub fn remove_picture(&mut self, index: usize) -> (Picture, PictureInformation) {
		self.pictures.remove(index)
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
	fn write_to(&mut self, file: &mut File) -> Result<()> {
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
		let mut temp = Cursor::new(Vec::new());
		super::write::create_pages(self, &mut temp, 0, false)?;

		writer.write_all(temp.get_ref())?;
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
	use crate::ogg::VorbisComments;
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

		let mut writer = vec![0, 0, 0, 0];
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
