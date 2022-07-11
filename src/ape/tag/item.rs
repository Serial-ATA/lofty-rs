use crate::ape::constants::INVALID_KEYS;
use crate::error::{FileDecodingError, LoftyError, Result};
use crate::file::FileType;
use crate::tag::item::{ItemValue, ItemValueRef, TagItem};
use crate::tag::TagType;

use std::convert::TryFrom;

#[derive(Debug, PartialEq, Eq, Clone)]
/// Represents an `APE` tag item
///
/// The restrictions for `APE` lie in the key rather than the value,
/// so these are still able to use [`ItemValue`]s
pub struct ApeItem {
	/// Whether or not to mark the item as read only
	pub read_only: bool,
	pub(crate) key: String,
	pub(crate) value: ItemValue,
}

impl ApeItem {
	/// Create an [`ApeItem`]
	///
	/// # Errors
	///
	/// * `key` is illegal ("ID3", "TAG", "OGGS", "MP+")
	/// * `key` has a bad length (must be 2 to 255, inclusive)
	/// * `key` contains invalid characters (must be in the range 0x20 to 0x7E, inclusive)
	pub fn new(key: String, value: ItemValue) -> Result<Self> {
		if INVALID_KEYS.contains(&&*key.to_uppercase()) {
			return Err(FileDecodingError::new(
				FileType::APE,
				"APE tag item contains an illegal key",
			)
			.into());
		}

		if !(2..=255).contains(&key.len()) {
			return Err(FileDecodingError::new(
				FileType::APE,
				"APE tag item key has an invalid length (< 2 || > 255)",
			)
			.into());
		}

		if key.chars().any(|c| !(0x20..=0x7E).contains(&(c as u32))) {
			return Err(FileDecodingError::new(
				FileType::APE,
				"APE tag item key contains invalid characters",
			)
			.into());
		}

		Ok(Self {
			read_only: false,
			key,
			value,
		})
	}

	/// Returns the item key
	pub fn key(&self) -> &str {
		&self.key
	}

	/// Returns the item value
	pub fn value(&self) -> &ItemValue {
		&self.value
	}

	// Used internally, has no correctness checks
	pub(crate) fn text(key: &str, value: String) -> Self {
		Self {
			read_only: false,
			key: String::from(key),
			value: ItemValue::Text(value),
		}
	}
}

impl TryFrom<TagItem> for ApeItem {
	type Error = LoftyError;

	fn try_from(value: TagItem) -> std::result::Result<Self, Self::Error> {
		Self::new(
			value
				.item_key
				.map_key(TagType::APE, false)
				.ok_or_else(|| {
					FileDecodingError::new(
						FileType::APE,
						"Attempted to convert an unsupported item key",
					)
				})?
				.to_string(),
			value.item_value,
		)
	}
}

pub(crate) struct ApeItemRef<'a> {
	pub read_only: bool,
	pub key: &'a str,
	pub value: ItemValueRef<'a>,
}

impl<'a> Into<ApeItemRef<'a>> for &'a ApeItem {
	fn into(self) -> ApeItemRef<'a> {
		ApeItemRef {
			read_only: self.read_only,
			key: self.key(),
			value: (&self.value).into(),
		}
	}
}
