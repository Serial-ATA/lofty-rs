use std::borrow::Cow;

use crate::tag::ItemValue;

/// The language of a [`SimpleTag`] or chapter
///
/// Notes:
///
/// - The default language is `Iso639_2("und")`
/// - ISO-639-2 was the original language code used in Matroska.
/// - BCP-47 is the newer, **recommended** language option.
/// - The ISO-639-2 language code allows for an optional country code, so the [Lang] type cannot be used.
///
/// [Lang]: crate::tag::items::Lang
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Language {
	/// An ISO-639-2 language code
	Iso639_2(String),
	/// A BCP-47 language code (recommended)
	Bcp47(String),
}

impl Default for Language {
	fn default() -> Self {
		Self::Iso639_2(String::from("und"))
	}
}

impl Language {
	/// Get the string value of the [`Language`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ebml::Language;
	///
	/// let lang = Language::Iso639_2(String::from("eng"));
	/// assert_eq!(lang.as_str(), "eng");
	/// ```
	pub fn as_str(&self) -> &str {
		match self {
			Self::Iso639_2(value) => value.as_str(),
			Self::Bcp47(value) => value.as_str(),
		}
	}
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TagValue<'a> {
	/// A UTF-8 string tag value
	String(Cow<'a, str>),
	/// A binary tag value
	Binary(Cow<'a, [u8]>),
}

impl From<TagValue<'_>> for ItemValue {
	fn from(value: TagValue<'_>) -> Self {
		match value {
			TagValue::String(s) => ItemValue::Text(s.into_owned()),
			TagValue::Binary(b) => ItemValue::Binary(b.into_owned()),
		}
	}
}

impl From<ItemValue> for TagValue<'_> {
	fn from(value: ItemValue) -> Self {
		match value {
			ItemValue::Text(s) | ItemValue::Locator(s) => TagValue::String(Cow::Owned(s)),
			ItemValue::Binary(b) => TagValue::Binary(Cow::Owned(b)),
		}
	}
}

impl From<String> for TagValue<'_> {
	fn from(value: String) -> Self {
		TagValue::String(value.into())
	}
}

impl<'a> From<Cow<'a, str>> for TagValue<'a> {
	fn from(value: Cow<'a, str>) -> Self {
		TagValue::String(value)
	}
}

impl<'a> From<&'a str> for TagValue<'a> {
	fn from(value: &'a str) -> Self {
		TagValue::String(Cow::Borrowed(value))
	}
}

impl From<Vec<u8>> for TagValue<'_> {
	fn from(value: Vec<u8>) -> Self {
		TagValue::Binary(value.into())
	}
}

impl<'a> From<Cow<'a, [u8]>> for TagValue<'a> {
	fn from(value: Cow<'a, [u8]>) -> Self {
		TagValue::Binary(value)
	}
}

impl<'a> From<&'a [u8]> for TagValue<'a> {
	fn from(value: &'a [u8]) -> Self {
		TagValue::Binary(Cow::Borrowed(value))
	}
}

impl TagValue<'_> {
	fn into_owned(self) -> TagValue<'static> {
		match self {
			TagValue::String(s) => TagValue::String(Cow::Owned(s.into_owned())),
			TagValue::Binary(b) => TagValue::Binary(Cow::Owned(b.into_owned())),
		}
	}
}

/// General information about the target
///
/// Notes on how `SimpleTag`s work:
/// - Their meaning depends on the [`Target`] of their parent [`Tag`]
/// - They **do not** need to have a value.
///
/// For more information, see [`Language`] and [`TagValue`]
///
/// [`Tag`]: crate::ebml::Tag
/// [`Target`]: crate::ebml::Target
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SimpleTag<'a> {
	/// The name of the tag as it is stored
	///
	/// This field can essentially contain anything, but the following conditions are recommended:
	///
	/// - It **SHOULD** consist of capital letters, numbers and the underscore character ‘_’.
	/// - It **SHOULD NOT** contain any space.
	///
	/// When in doubt, the [`TagName`] enum can be used, which covers all specified tags.
	pub name: Cow<'a, str>,
	/// The language of the tag
	///
	/// See [`Language`] for more information.
	pub language: Option<Language>,
	/// Whether [`language`] is the default/original language to use
	///
	/// This is used when multiple languages are present in a file. Otherwise, this
	/// will always be `true`.
	///
	/// [`language`]: #structfield.language
	pub default: bool,
	/// The actual tag value
	///
	/// For more information, see [`TagValue`]
	pub value: Option<TagValue<'a>>,
}

impl<'a> SimpleTag<'a> {
	/// Create a new `SimpleTag` with the given name and value
	///
	/// # Example
	///
	/// ```
	/// use lofty::ebml::{SimpleTag, TagName, TagValue};
	///
	/// let tag = SimpleTag::new(TagName::Title, "My Title");
	/// ```
	pub fn new<N, V>(name: N, value: V) -> Self
	where
		N: Into<Cow<'a, str>>,
		V: Into<TagValue<'a>>,
	{
		Self {
			name: name.into(),
			language: None,
			default: false,
			value: Some(value.into()),
		}
	}

	/// Get the value of the `SimpleTag` if it is [`TagValue::String`]
	///
	/// # Example
	///
	/// ```rust
	/// use lofty::ebml::{SimpleTag, TagName, TagValue};
	///
	/// let tag = SimpleTag::new(TagName::Title, "My Title");
	/// assert_eq!(tag.get_str(), Some("My Title"));
	/// ```
	pub fn get_str(&'a self) -> Option<&'a str> {
		match &self.value {
			Some(TagValue::String(s)) => Some(s),
			_ => None,
		}
	}

	/// Get the value of the `SimpleTag` if it is [`TagValue::Binary`]
	///
	/// # Example
	///
	/// ```rust
	/// use lofty::ebml::{SimpleTag, TagName, TagValue};
	///
	/// let tag = SimpleTag::new(TagName::Title, vec![1, 2, 3]);
	/// assert_eq!(tag.get_binary(), Some(&[1, 2, 3][..]));
	/// ```
	pub fn get_binary(&'a self) -> Option<&'a [u8]> {
		match &self.value {
			Some(TagValue::Binary(b)) => Some(b),
			_ => None,
		}
	}

	pub(crate) fn into_owned(self) -> SimpleTag<'static> {
		SimpleTag {
			name: Cow::Owned(self.name.into_owned()),
			language: self.language,
			default: self.default,
			value: self.value.map(TagValue::into_owned),
		}
	}
}

// Used in conversions
pub(super) const TOMBSTONE_SIMPLE_TAG: SimpleTag<'static> = SimpleTag {
	name: Cow::Borrowed(""),
	language: None,
	default: false,
	value: None,
};