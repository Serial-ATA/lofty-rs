pub(in crate::logic::iff::wav) mod read;
pub(in crate::logic::iff::wav) mod write;

use crate::error::Result;
use crate::types::item::{ItemKey, ItemValue, TagItem};
use crate::types::tag::{Tag, TagType};

use std::fs::File;

#[derive(Default)]
/// A RIFF INFO LIST
pub struct RiffInfoList {
	/// A collection of chunk-value pairs
	pub(crate) items: Vec<(String, String)>,
}

impl RiffInfoList {
	pub fn push(&mut self, key: String, value: String) {
		if valid_key(key.as_str()) {
			self.items.push((key, value))
		}
	}

	pub fn remove(&mut self, key: &str) {
		self.items
			.iter()
			.position(|(k, _)| k == key)
			.map(|p| self.items.remove(p));
	}

	pub fn items(&self) -> &[(String, String)] {
		self.items.as_slice()
	}
}

impl RiffInfoList {
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
