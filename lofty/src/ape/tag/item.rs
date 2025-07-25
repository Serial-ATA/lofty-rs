use crate::ape::constants::INVALID_KEYS;
use crate::error::{LoftyError, Result};
use crate::macros::decode_err;
use crate::tag::item::ItemValueRef;
use crate::tag::{ItemValue, TagItem, TagType};

/// Represents an `APE` tag item
///
/// The restrictions for `APE` lie in the key rather than the value,
/// so these are still able to use [`ItemValue`]s
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ApeItem {
	/// Whether or not to mark the item as read only
	pub read_only: bool,
	pub(crate) key: String,
	pub(crate) value: ItemValue,
}

impl ApeItem {
	/// Placeholder for conversions
	pub(crate) const EMPTY: Self = ApeItem {
		read_only: false,
		key: String::new(),
		value: ItemValue::Text(String::new()),
	};

	/// Create an [`ApeItem`]
	///
	/// # Errors
	///
	/// * `key` is illegal ("ID3", "TAG", "OGGS", "MP+")
	/// * `key` has a bad length (must be 2 to 255, inclusive)
	/// * `key` contains invalid characters (must be in the range 0x20 to 0x7E, inclusive)
	pub fn new(key: String, value: ItemValue) -> Result<Self> {
		if INVALID_KEYS.contains(&&*key.to_uppercase()) {
			decode_err!(@BAIL Ape, "APE tag item contains an illegal key");
		}

		if !(2..=255).contains(&key.len()) {
			decode_err!(@BAIL Ape, "APE tag item key has an invalid length (< 2 || > 255)");
		}

		if key.chars().any(|c| !(0x20..=0x7E).contains(&(c as u32))) {
			decode_err!(@BAIL Ape, "APE tag item key contains invalid characters");
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
				.map_key(TagType::Ape)
				.ok_or_else(|| decode_err!(Ape, "Attempted to convert an unsupported item key"))?
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
