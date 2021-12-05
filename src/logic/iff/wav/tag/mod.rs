pub(in crate::logic::iff::wav) mod read;
pub(in crate::logic::iff::wav) mod write;

use crate::error::Result;
use crate::types::item::{ItemKey, ItemValue, TagItem};
use crate::types::tag::{Tag, TagType};

use std::fs::File;
use std::io::{Read, Seek};

#[derive(Default, Debug, PartialEq)]
/// A RIFF INFO LIST
pub struct RiffInfoList {
	/// A collection of chunk-value pairs
	pub(crate) items: Vec<(String, String)>,
}

impl RiffInfoList {
	/// Get an item by key
	pub fn get(&self, key: &str) -> Option<&str> {
		self.items
			.iter()
			.find(|(k, _)| k == key)
			.map(|(_, v)| v.as_str())
	}

	/// Insert an item
	///
	/// NOTE: This will do nothing if `key` is not 4 bytes in length and entirely ascii characters
	///
	/// This will case-insensitively replace any item with the same key
	pub fn insert(&mut self, key: String, value: String) {
		if valid_key(key.as_str()) {
			self.items
				.iter()
				.position(|(k, _)| k.eq_ignore_ascii_case(key.as_str()))
				.map(|p| self.items.remove(p));
			self.items.push((key, value))
		}
	}

	/// Remove an item by key
	///
	/// This will case-insensitively remove an item with the key
	pub fn remove(&mut self, key: &str) {
		self.items
			.iter()
			.position(|(k, _)| k.eq_ignore_ascii_case(key))
			.map(|p| self.items.remove(p));
	}

	/// Returns the tag's items in (key, value) pairs
	pub fn items(&self) -> &[(String, String)] {
		self.items.as_slice()
	}
}

impl RiffInfoList {
	#[allow(clippy::missing_errors_doc)]
	/// Parses a [`RiffInfoList`] from a reader
	///
	/// NOTE: This is **NOT** for reading from a file.
	/// This is used internally, and requires the end position be provided.
	pub fn read_from<R>(reader: &mut R, end: u64) -> Result<Self>
	where
		R: Read + Seek,
	{
		let mut tag = Self::default();

		read::parse_riff_info(reader, end, &mut tag)?;

		Ok(tag)
	}

	/// Writes the tag to a file
	///
	/// # Errors
	///
	/// * Attempting to write the tag to a format that does not support it
	pub fn write_to(&self, file: &mut File) -> Result<()> {
		Into::<RiffInfoListRef>::into(self).write_to(file)
	}
}

impl From<RiffInfoList> for Tag {
	fn from(input: RiffInfoList) -> Self {
		let mut tag = Tag::new(TagType::RiffInfo);

		for (k, v) in input.items {
			let item_key = ItemKey::from_key(&TagType::RiffInfo, &k);

			tag.insert_item_unchecked(TagItem::new(
				item_key,
				ItemValue::Text(v.trim_matches('\0').to_string()),
			));
		}

		tag
	}
}

impl From<Tag> for RiffInfoList {
	fn from(input: Tag) -> Self {
		let mut riff_info = RiffInfoList::default();

		for item in input.items {
			if let ItemValue::Text(val) | ItemValue::Locator(val) = item.item_value {
				let item_key = match item.item_key {
					ItemKey::Unknown(unknown) => {
						if unknown.len() == 4 && unknown.is_ascii() {
							unknown.to_string()
						} else {
							continue;
						}
					},
					// Safe to unwrap since we already checked ItemKey::Unknown
					k => k.map_key(&TagType::RiffInfo, false).unwrap().to_string(),
				};

				riff_info.items.push((item_key, val))
			}
		}

		riff_info
	}
}

pub(crate) struct RiffInfoListRef<'a> {
	items: Box<dyn Iterator<Item = (&'a str, &'a String)> + 'a>,
}

impl<'a> Into<RiffInfoListRef<'a>> for &'a RiffInfoList {
	fn into(self) -> RiffInfoListRef<'a> {
		RiffInfoListRef {
			items: Box::new(self.items.iter().map(|(k, v)| (k.as_str(), v))),
		}
	}
}

impl<'a> Into<RiffInfoListRef<'a>> for &'a Tag {
	fn into(self) -> RiffInfoListRef<'a> {
		RiffInfoListRef {
			items: Box::new(self.items.iter().filter_map(|i| {
				if let ItemValue::Text(val) | ItemValue::Locator(val) = &i.item_value {
					let item_key = i.key().map_key(&TagType::RiffInfo, true).unwrap();

					if item_key.len() == 4 && item_key.is_ascii() {
						Some((item_key, val))
					} else {
						None
					}
				} else {
					None
				}
			})),
		}
	}
}

impl<'a> RiffInfoListRef<'a> {
	pub(crate) fn write_to(&mut self, file: &mut File) -> Result<()> {
		write::write_riff_info(file, self)
	}
}

fn valid_key(key: &str) -> bool {
	key.len() == 4 && key.is_ascii()
}
