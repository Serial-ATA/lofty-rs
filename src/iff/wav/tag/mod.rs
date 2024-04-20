pub(super) mod read;
mod write;

use crate::config::WriteOptions;
use crate::error::{LoftyError, Result};
use crate::tag::{
	try_parse_year, Accessor, ItemKey, ItemValue, MergeTag, SplitTag, Tag, TagExt, TagItem, TagType,
};
use crate::util::io::{FileLike, Length, Truncate};

use std::borrow::Cow;
use std::io::Write;

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

/// ## Conversions
///
/// ### To `Tag`
///
/// All items will be converted to a [`TagItem`], with all unknown keys being stored with [`ItemKey::Unknown`].
///
/// ### From `Tag`
///
/// When converting a [`TagItem`], two conditions must be met:
///
/// * The [`TagItem`] has a value other than [`ItemValue::Binary`](crate::ItemValue::Binary)
/// * It has a key that is 4 bytes in length and within the ASCII range
#[derive(Default, Debug, PartialEq, Eq, Clone)]
#[tag(description = "A RIFF INFO LIST", supported_formats(Wav))]
pub struct RIFFInfoList {
	/// A collection of chunk-value pairs
	pub(crate) items: Vec<(String, String)>,
}

impl RIFFInfoList {
	/// Create a new empty `RIFFInfoList`
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::iff::wav::RIFFInfoList;
	/// use lofty::tag::TagExt;
	///
	/// let riff_info_tag = RIFFInfoList::new();
	/// assert!(riff_info_tag.is_empty());
	/// ```
	pub fn new() -> Self {
		Self::default()
	}

	/// Get an item by key
	pub fn get(&self, key: &str) -> Option<&str> {
		self.items
			.iter()
			.find(|(k, _)| k == key)
			.map(|(_, v)| v.as_str())
	}

	/// Insert an item
	///
	/// NOTE: This will do nothing if `key` is invalid
	///
	/// This will case-insensitively replace any item with the same key
	pub fn insert(&mut self, key: String, value: String) {
		if read::verify_key(key.as_str()) {
			self.items
				.iter()
				.position(|(k, _)| k.eq_ignore_ascii_case(key.as_str()))
				.map(|p| self.items.remove(p));
			self.items.push((key, value))
		}
	}

	/// Remove an item by key
	///
	/// This will case-insensitively remove an item with the key, returning it
	/// if it exists.
	pub fn remove(&mut self, key: &str) -> Option<String> {
		if let Some((_, value)) = self
			.items
			.iter()
			.position(|(k, _)| k.eq_ignore_ascii_case(key))
			.map(|p| self.items.remove(p))
		{
			return Some(value);
		}

		None
	}
}

impl Accessor for RIFFInfoList {
	impl_accessor!(
		artist  => "IART";
		title   => "INAM";
		album   => "IPRD";
		genre   => "IGNR";
		comment => "ICMT";
	);

	fn track(&self) -> Option<u32> {
		if let Some(item) = self.get("IPRT") {
			return item.parse::<u32>().ok();
		}

		None
	}

	fn set_track(&mut self, value: u32) {
		self.insert(String::from("IPRT"), value.to_string());
	}

	fn remove_track(&mut self) {
		self.remove("IPRT");
	}

	fn track_total(&self) -> Option<u32> {
		if let Some(item) = self.get("IFRM") {
			return item.parse::<u32>().ok();
		}

		None
	}

	fn set_track_total(&mut self, value: u32) {
		self.insert(String::from("IFRM"), value.to_string());
	}

	fn remove_track_total(&mut self) {
		self.remove("IFRM");
	}

	fn year(&self) -> Option<u32> {
		if let Some(item) = self.get("ICRD") {
			return try_parse_year(item);
		}

		None
	}

	fn set_year(&mut self, value: u32) {
		self.insert(String::from("ICRD"), value.to_string());
	}

	fn remove_year(&mut self) {
		let _ = self.remove("ICRD");
	}
}

impl IntoIterator for RIFFInfoList {
	type Item = (String, String);
	type IntoIter = std::vec::IntoIter<Self::Item>;

	fn into_iter(self) -> Self::IntoIter {
		self.items.into_iter()
	}
}

impl<'a> IntoIterator for &'a RIFFInfoList {
	type Item = &'a (String, String);
	type IntoIter = std::slice::Iter<'a, (String, String)>;

	fn into_iter(self) -> Self::IntoIter {
		self.items.iter()
	}
}

impl TagExt for RIFFInfoList {
	type Err = LoftyError;
	type RefKey<'a> = &'a str;

	#[inline]
	fn tag_type(&self) -> TagType {
		TagType::RiffInfo
	}

	fn len(&self) -> usize {
		self.items.len()
	}

	fn contains<'a>(&'a self, key: Self::RefKey<'a>) -> bool {
		self.items
			.iter()
			.any(|(item_key, _)| item_key.eq_ignore_ascii_case(key))
	}

	fn is_empty(&self) -> bool {
		self.items.is_empty()
	}

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
		RIFFInfoListRef::new(self.items.iter().map(|(k, v)| (k.as_str(), v.as_str())))
			.write_to(file, write_options)
	}

	fn dump_to<W: Write>(
		&self,
		writer: &mut W,
		write_options: WriteOptions,
	) -> std::result::Result<(), Self::Err> {
		RIFFInfoListRef::new(self.items.iter().map(|(k, v)| (k.as_str(), v.as_str())))
			.dump_to(writer, write_options)
	}

	fn clear(&mut self) {
		self.items.clear();
	}
}

#[derive(Debug, Clone, Default)]
pub struct SplitTagRemainder;

impl SplitTag for RIFFInfoList {
	type Remainder = SplitTagRemainder;

	fn split_tag(self) -> (Self::Remainder, Tag) {
		(SplitTagRemainder, self.into())
	}
}

impl MergeTag for SplitTagRemainder {
	type Merged = RIFFInfoList;

	fn merge_tag(self, tag: Tag) -> Self::Merged {
		tag.into()
	}
}

impl From<RIFFInfoList> for Tag {
	fn from(input: RIFFInfoList) -> Self {
		let mut tag = Self::new(TagType::RiffInfo);

		for (k, v) in input.items {
			let item_key = ItemKey::from_key(TagType::RiffInfo, &k);

			tag.items.push(TagItem::new(
				item_key,
				ItemValue::Text(v.trim_matches('\0').to_string()),
			));
		}

		tag
	}
}

impl From<Tag> for RIFFInfoList {
	fn from(input: Tag) -> Self {
		let mut riff_info = RIFFInfoList::default();

		for item in input.items {
			if let ItemValue::Text(val) | ItemValue::Locator(val) = item.item_value {
				match item.item_key {
					ItemKey::Unknown(unknown) => {
						if read::verify_key(&unknown) {
							riff_info.items.push((unknown, val))
						}
					},
					k => {
						if let Some(key) = k.map_key(TagType::RiffInfo, false) {
							riff_info.items.push((key.to_string(), val))
						}
					},
				}
			}
		}

		riff_info
	}
}

pub(crate) struct RIFFInfoListRef<'a, I>
where
	I: Iterator<Item = (&'a str, &'a str)>,
{
	pub(crate) items: I,
}

impl<'a, I> RIFFInfoListRef<'a, I>
where
	I: Iterator<Item = (&'a str, &'a str)>,
{
	pub(crate) fn new(items: I) -> RIFFInfoListRef<'a, I> {
		RIFFInfoListRef { items }
	}

	pub(crate) fn write_to<F>(&mut self, file: &mut F, write_options: WriteOptions) -> Result<()>
	where
		F: FileLike,
		LoftyError: From<<F as Truncate>::Error>,
		LoftyError: From<<F as Length>::Error>,
	{
		write::write_riff_info(file, self, write_options)
	}

	pub(crate) fn dump_to<W: Write>(
		&mut self,
		writer: &mut W,
		_write_options: WriteOptions,
	) -> Result<()> {
		let mut temp = Vec::new();
		write::create_riff_info(&mut self.items, &mut temp)?;

		writer.write_all(&temp)?;

		Ok(())
	}
}

pub(crate) fn tagitems_into_riff<'a>(
	items: impl IntoIterator<Item = &'a TagItem>,
) -> impl Iterator<Item = (&'a str, &'a str)> {
	items.into_iter().filter_map(|i| {
		let item_key = i.key().map_key(TagType::RiffInfo, true);

		match (item_key, i.value()) {
			(Some(key), ItemValue::Text(val) | ItemValue::Locator(val))
				if read::verify_key(key) =>
			{
				Some((key, val.as_str()))
			},
			_ => None,
		}
	})
}

#[cfg(test)]
mod tests {
	use crate::config::WriteOptions;
	use crate::iff::chunk::Chunks;
	use crate::iff::wav::RIFFInfoList;
	use crate::prelude::*;
	use crate::tag::{Tag, TagType};

	use byteorder::LittleEndian;

	use std::io::Cursor;

	#[test]
	fn parse_riff_info() {
		let mut expected_tag = RIFFInfoList::default();

		expected_tag.insert(String::from("IART"), String::from("Bar artist"));
		expected_tag.insert(String::from("ICMT"), String::from("Qux comment"));
		expected_tag.insert(String::from("ICRD"), String::from("1984"));
		expected_tag.insert(String::from("INAM"), String::from("Foo title"));
		expected_tag.insert(String::from("IPRD"), String::from("Baz album"));
		expected_tag.insert(String::from("IPRT"), String::from("1"));

		let tag = crate::tag::utils::test_utils::read_path("tests/tags/assets/test.riff");
		let mut parsed_tag = RIFFInfoList::default();

		super::read::parse_riff_info(
			&mut Cursor::new(&tag[..]),
			&mut Chunks::<LittleEndian>::new(tag.len() as u64),
			(tag.len() - 1) as u64,
			&mut parsed_tag,
		)
		.unwrap();

		assert_eq!(expected_tag, parsed_tag);
	}

	#[test]
	fn riff_info_re_read() {
		let tag = crate::tag::utils::test_utils::read_path("tests/tags/assets/test.riff");
		let mut parsed_tag = RIFFInfoList::default();

		super::read::parse_riff_info(
			&mut Cursor::new(&tag[..]),
			&mut Chunks::<LittleEndian>::new(tag.len() as u64),
			(tag.len() - 1) as u64,
			&mut parsed_tag,
		)
		.unwrap();

		let mut writer = Vec::new();
		parsed_tag
			.dump_to(&mut writer, WriteOptions::default())
			.unwrap();

		let mut temp_parsed_tag = RIFFInfoList::default();

		// Remove the LIST....INFO from the tag
		super::read::parse_riff_info(
			&mut Cursor::new(&writer[12..]),
			&mut Chunks::<LittleEndian>::new(tag.len() as u64),
			(tag.len() - 13) as u64,
			&mut temp_parsed_tag,
		)
		.unwrap();

		assert_eq!(parsed_tag, temp_parsed_tag);
	}

	#[test]
	fn riff_info_to_tag() {
		let tag_bytes = crate::tag::utils::test_utils::read_path("tests/tags/assets/test.riff");

		let mut reader = std::io::Cursor::new(&tag_bytes[..]);
		let mut riff_info = RIFFInfoList::default();

		super::read::parse_riff_info(
			&mut reader,
			&mut Chunks::<LittleEndian>::new(tag_bytes.len() as u64),
			(tag_bytes.len() - 1) as u64,
			&mut riff_info,
		)
		.unwrap();

		let tag: Tag = riff_info.into();

		crate::tag::utils::test_utils::verify_tag(&tag, true, false);
	}

	#[test]
	fn tag_to_riff_info() {
		let tag = crate::tag::utils::test_utils::create_tag(TagType::RiffInfo);

		let riff_info: RIFFInfoList = tag.into();

		assert_eq!(riff_info.get("INAM"), Some("Foo title"));
		assert_eq!(riff_info.get("IART"), Some("Bar artist"));
		assert_eq!(riff_info.get("IPRD"), Some("Baz album"));
		assert_eq!(riff_info.get("ICMT"), Some("Qux comment"));
		assert_eq!(riff_info.get("IPRT"), Some("1"));
	}
}
