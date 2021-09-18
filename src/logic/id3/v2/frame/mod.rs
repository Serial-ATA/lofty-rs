pub(in crate::logic::id3::v2) mod content;
mod header;
pub(in crate::logic::id3::v2) mod read;

use super::util::text_utils::TextEncoding;

#[derive(PartialEq, Clone, Debug, Eq, Hash)]
/// Information about an ID3v2 frame that requires a language
pub struct LanguageSpecificFrame {
	/// The encoding of the description and comment text
	pub encoding: TextEncoding,
	/// ISO-639-2 language code (3 bytes)
	pub language: String,
	/// Unique content description
	pub description: Option<String>,
}

#[derive(PartialEq, Clone, Debug, Eq, Hash)]
/// Different types of ID3v2 frames that require varying amounts of information
pub enum Id3v2Frame {
	/// Represents a "COMM" frame
	///
	/// Due to the amount of information needed, it is contained in a separate struct, [`LanguageSpecificFrame`]
	Comment(LanguageSpecificFrame),
	/// Represents a "USLT" frame
	///
	/// Due to the amount of information needed, it is contained in a separate struct, [`LanguageSpecificFrame`]
	UnSyncText(LanguageSpecificFrame),
	/// Represents a "T..." (excluding TXXX) frame
	///
	/// NOTE: Text frame names **must** be unique
	///
	/// This can be thought of as Text(name, encoding)
	Text(String, TextEncoding),
	/// Represents a "TXXX" frame
	///
	/// This can be thought of as TXXX(encoding, description), as TXXX frames are often identified by descriptions.
	UserText(TextEncoding, String),
	/// Represents a "W..." (excluding WXXX) frame
	///
	/// NOTES:
	///
	/// * This is a fallback if there was no [`ItemKey`](crate::ItemKey) mapping
	/// * URL frame names **must** be unique
	///
	/// No encoding needs to be provided as all URLs are [`TextEncoding::Latin1`]
	URL(String),
	/// Represents a "WXXX" frame
	///
	/// This can be thought of as WXXX(encoding, description), as WXXX frames are often identified by descriptions.
	UserURL(TextEncoding, String),
	/// Represents a "SYLT" frame
	///
	/// Nothing is required here, the entire frame is stored as [`ItemValue::Binary`](crate::ItemValue::Binary). For parsing see [`SynchronizedText::parse`](crate::id3::v2::SynchronizedText::parse)
	SyncText,
	/// Represents a "GEOB" frame
	///
	/// Nothing is required here, the entire frame is stored as [`ItemValue::Binary`](crate::ItemValue::Binary). For parsing see [`GeneralEncapsulatedObject::parse`](crate::id3::v2::GeneralEncapsulatedObject::parse)
	EncapsulatedObject,
	/// When an ID3v2.2 key couldn't be upgraded
	///
	/// This **will not** be written. It is up to the user to upgrade and store the key as another variant.
	Outdated(String),
}
