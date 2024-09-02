use crate::tag::ItemValue;

/// The language of a [`SimpleTag`] or chapter
///
/// Notes:
///
/// - ISO-639-2 was the original language code used in Matroska.
/// - BCP-47 is the newer, **recommended** language option.
/// - The ISO-639-2 language code allows for an optional country code, so the [Lang] type cannot be used.
///
/// [Lang]: crate::tag::items::Lang
pub enum Language {
	/// An ISO-639-2 language code
	Iso639_2(String),
	/// A BCP-47 language code (recommended)
	Bcp47(String),
}

/// The type of content stored in a [`SimpleTag`]
///
/// Matroska allows two different types of content to be stored in tags: UTF-8 strings and binary data.
///
/// ## Conversions with [`ItemValue`]
///
/// A `TagValue` can be converted to and from an [`ItemValue`] with the following conversions:
///
/// ### To [`ItemValue`]
///
/// - [`TagValue::String`] -> [`ItemValue::Text`]
/// - [`TagValue::Binary`] -> [`ItemValue::Binary`]
///
/// ### From [`ItemValue`]
///
/// - [`ItemValue::Text`] | [`ItemValue::Locator`] -> [`TagValue::String`]
/// - [`ItemValue::Binary`] -> [`TagValue::Binary`]
pub enum TagValue {
	/// A UTF-8 string tag value
	String(String),
	/// A binary tag value
	Binary(Vec<u8>),
}

impl From<TagValue> for ItemValue {
	fn from(value: TagValue) -> Self {
		match value {
			TagValue::String(s) => ItemValue::Text(s),
			TagValue::Binary(b) => ItemValue::Binary(b),
		}
	}
}

impl From<ItemValue> for TagValue {
	fn from(value: ItemValue) -> Self {
		match value {
			ItemValue::Text(s) | ItemValue::Locator(s) => TagValue::String(s),
			ItemValue::Binary(b) => TagValue::Binary(b),
		}
	}
}

/// General information about the target
///
/// Notes on how `SimpleTag`s work:
///
/// - Multiple [`SimpleTag`]s can exist in a file.
/// - They each describe a single [`Target`].
///   - This also means that multiple tags can describe the same target.
/// - They **do not** need to have a value.
pub struct SimpleTag {
	/// The name of the tag as it is stored
	///
	/// This field can essentially contain anything, but the following conditions are recommended:
	///
	/// - It **SHOULD** consist of capital letters, numbers and the underscore character ‘_’.
	/// - It **SHOULD NOT** contain any space.
	///
	/// When in doubt, the [`TagName`] enum can be used, which covers all specified tags.
	pub name: String,
	/// The language of the tag
	///
	/// See [`Language`] for more information.
	pub language: Option<Language>,
	/// Whether [`language`] is the default/original langauge to use
	///
	/// This is used when multiple languages are present in a file. This field will be ignored
	/// if [`language`] is `None`.
	///
	/// [`language`]: #structfield.language
	pub default: bool,
	/// The actual tag value
	///
	/// For more information, see [`TagValue`]
	pub value: Option<TagValue>,
}
