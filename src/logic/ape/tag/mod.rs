mod item;
pub(in crate::logic) mod read;
pub(in crate::logic) mod write;

use crate::error::Result;
use crate::logic::ape::tag::item::{ApeItem, ApeItemRef};
use crate::types::item::{ItemKey, ItemValue, TagItem};
use crate::types::tag::{Tag, TagType};

use std::collections::HashMap;
use std::convert::TryInto;
use std::fs::File;

#[derive(Default)]
/// An APE tag
pub struct ApeTag {
	pub read_only: bool,
	pub(super) items: HashMap<String, ApeItem>,
}

impl ApeTag {
	pub fn get_key(&self, key: &str) -> Option<&ApeItem> {
		self.items.get(key)
	}

	pub fn push_item(&mut self, value: ApeItem) {
		let _ = self.items.insert(value.key.clone(), value);
	}

	pub fn remove_key(&mut self, key: &str) {
		let _ = self.items.remove(key);
	}
}

impl ApeTag {
	pub fn write_to(&self, file: &mut File) -> Result<()> {
		Into::<ApeTagRef>::into(self).write_to(file)
	}
}

impl From<ApeTag> for Tag {
	fn from(input: ApeTag) -> Self {
		let mut tag = Tag::new(TagType::Ape);

		for (_, item) in input.items {
			let item = TagItem::new(ItemKey::from_key(&TagType::Ape, &*item.key), item.value);

			tag.insert_item_unchecked(item)
		}

		tag
	}
}

impl From<Tag> for ApeTag {
	fn from(input: Tag) -> Self {
		let mut ape_tag = Self::default();

		for item in input.items {
			if let Ok(ape_item) = item.try_into() {
				ape_tag.push_item(ape_item)
			}
		}

		for pic in input.pictures {
			if let Some(key) = pic.pic_type.as_ape_key() {
				if let Ok(item) =
					ApeItem::new(key.to_string(), ItemValue::Binary(pic.as_ape_bytes()))
				{
					ape_tag.push_item(item)
				}
			}
		}

		ape_tag
	}
}

pub(in crate::logic) struct ApeTagRef<'a> {
	read_only: bool,
	pub(super) items: HashMap<&'a str, ApeItemRef<'a>>,
}

impl<'a> ApeTagRef<'a> {
	pub(crate) fn write_to(&self, file: &mut File) -> Result<()> {
		write::write_to(file, self)
	}
}

impl<'a> Into<ApeTagRef<'a>> for &'a Tag {
	fn into(self) -> ApeTagRef<'a> {
		let mut items = HashMap::<&'a str, ApeItemRef<'a>>::new();

		for item in &self.items {
			let key = item.key().map_key(&TagType::Ape, true).unwrap();

			items.insert(
				key,
				ApeItemRef {
					read_only: false,
					value: (&item.item_value).into(),
				},
			);
		}

		ApeTagRef {
			read_only: false,
			items,
		}
	}
}

impl<'a> Into<ApeTagRef<'a>> for &'a ApeTag {
	fn into(self) -> ApeTagRef<'a> {
		ApeTagRef {
			read_only: self.read_only,
			items: {
				let mut items = HashMap::<&str, ApeItemRef<'a>>::new();

				for (k, v) in &self.items {
					items.insert(k.as_str(), v.into());
				}

				items
			},
		}
	}
}
