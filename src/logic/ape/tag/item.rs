use crate::error::{LoftyError, Result};
use crate::logic::ape::constants::INVALID_KEYS;
use crate::types::item::{ItemValue, ItemValueRef, TagItem};
use crate::types::tag::TagType;

use std::convert::TryFrom;

pub struct ApeItem {
	pub read_only: bool,
	pub(crate) key: String,
	pub(crate) value: ItemValue,
}

impl ApeItem {
	pub fn new(key: String, value: ItemValue) -> Result<Self> {
		if INVALID_KEYS.contains(&&*key.to_uppercase()) {
			return Err(LoftyError::Ape("Tag item contains an illegal key"));
		}

		if key.chars().any(|c| !c.is_ascii()) {
			return Err(LoftyError::Ape("Tag item contains a non ASCII key"));
		}

		Ok(Self {
			read_only: false,
			key,
			value,
		})
	}

	pub fn set_read_only(&mut self) {
		self.read_only = true
	}
}

impl TryFrom<TagItem> for ApeItem {
	type Error = LoftyError;

	fn try_from(value: TagItem) -> std::prelude::rust_2015::Result<Self, Self::Error> {
		Self::new(
			value
				.item_key
				.map_key(&TagType::Ape, false)
				.ok_or(LoftyError::Ape(
					"Attempted to convert an unsupported item key",
				))?
				.to_string(),
			value.item_value,
		)
	}
}

pub(in crate::logic) struct ApeItemRef<'a> {
	pub read_only: bool,
	pub value: ItemValueRef<'a>,
}

impl<'a> Into<ApeItemRef<'a>> for &'a ApeItem {
	fn into(self) -> ApeItemRef<'a> {
		ApeItemRef {
			read_only: self.read_only,
			value: (&self.value).into(),
		}
	}
}
