pub(in crate::logic::id3::v2) mod content;
mod header;
pub(in crate::logic::id3::v2) mod read;

use super::util::text_utils::TextEncoding;
use crate::error::{LoftyError, Result};
use crate::logic::id3::v2::util::text_utils::encode_text;
use crate::logic::id3::v2::util::upgrade::{upgrade_v2, upgrade_v3};
use crate::types::item::{ItemKey, ItemValue, TagItem};
use crate::types::picture::Picture;
use crate::types::tag::TagType;
use std::convert::TryFrom;

#[derive(PartialEq, Clone, Debug, Eq, Hash)]
pub struct Frame {
	pub(super) id: FrameID,
	pub(super) value: FrameValue,
	pub(super) flags: FrameFlags,
}

impl Frame {
	pub fn new(id: &str, value: FrameValue, flags: FrameFlags) -> Result<Self> {
		let id = match id.len() {
			// An ID with a length of 4 could be either V3 or V4.
			4 => match upgrade_v3(id) {
				None => FrameID::Valid(id.to_string()),
				Some(id) => FrameID::Valid(id.to_string()),
			},
			3 => match upgrade_v2(id) {
				None => FrameID::Outdated(id.to_string()),
				Some(upgraded) => FrameID::Valid(upgraded.to_string()),
			},
			_ => {
				return Err(LoftyError::Id3v2(
					"Frame ID has a bad length (!= 3 || != 4)",
				))
			}
		};

		match id {
			FrameID::Valid(id) | FrameID::Outdated(id) if !id.is_ascii() => {
				return Err(LoftyError::Id3v2("Frame ID contains non-ascii characters"))
			}
			_ => {}
		}

		Ok(Self { id, value, flags })
	}

	pub fn id_str(&self) -> &str {
		match &self.id {
			FrameID::Valid(id) | FrameID::Outdated(id) => id.as_str(),
		}
	}

	pub fn content(&self) -> &FrameValue {
		&self.value
	}

	/// Returns a reference to the [`FrameFlags`]
	pub fn flags(&self) -> &FrameFlags {
		&self.flags
	}

	/// Set the item's flags
	pub fn set_flags(&mut self, flags: FrameFlags) {
		self.flags = flags
	}
}

#[derive(PartialEq, Clone, Debug, Eq, Hash)]
/// Information about an ID3v2 frame that requires a language
pub struct LanguageFrame {
	/// The encoding of the description and comment text
	pub encoding: TextEncoding,
	/// ISO-639-2 language code (3 bytes)
	pub language: String,
	/// Unique content description
	pub description: String,
	/// The actual frame content
	pub content: String,
}

impl LanguageFrame {
	pub fn as_bytes(&self) -> Result<Vec<u8>> {
		let mut bytes = vec![self.encoding as u8];

		if self.language.len() != 3 || !self.language.is_ascii() {
			return Err(LoftyError::Id3v2(
				"Invalid frame language found (expected 3 ascii characters)",
			));
		}

		bytes.extend(self.language.as_bytes().iter());
		bytes.extend(encode_text(&*self.description, self.encoding, true).iter());
		bytes.extend(encode_text(&*self.content, self.encoding, false));

		Ok(bytes)
	}
}

#[derive(PartialEq, Clone, Debug, Eq, Hash)]
pub struct EncodedTextFrame {
	/// The encoding of the description and comment text
	pub encoding: TextEncoding,
	/// Unique content description
	pub description: String,
	/// The actual frame content
	pub content: String,
}

impl EncodedTextFrame {
	pub fn as_bytes(&self) -> Vec<u8> {
		let mut bytes = vec![self.encoding as u8];

		bytes.extend(encode_text(&*self.description, self.encoding, true).iter());
		bytes.extend(encode_text(&*self.content, self.encoding, false));

		bytes
	}
}

#[derive(PartialEq, Clone, Debug, Eq, Hash)]
/// Different types of ID3v2 frames that require varying amounts of information
pub enum FrameID {
	Valid(String),
	/// When an ID3v2.2 key couldn't be upgraded
	///
	/// This **will not** be written. It is up to the user to upgrade and store the key as [`Id3v2Frame::Valid`](Self::Valid).
	///
	/// The entire frame is stored as [`ItemValue::Binary`](crate::ItemValue::Binary).
	Outdated(String),
}

impl TryFrom<ItemKey> for FrameID {
	type Error = LoftyError;

	fn try_from(value: ItemKey) -> std::prelude::rust_2015::Result<Self, Self::Error> {
		match value {
			ItemKey::Unknown(unknown) if unknown.len() == 4 && unknown.is_ascii() => {
				Ok(Self::Valid(unknown.to_ascii_uppercase()))
			}
			k => k.map_key(&TagType::Id3v2, false).map_or(
				Err(LoftyError::Id3v2(
					"ItemKey does not meet the requirements to be a FrameID",
				)),
				|id| Ok(Self::Valid(id.to_string())),
			),
		}
	}
}

#[derive(PartialEq, Clone, Debug, Eq, Hash)]
pub enum FrameValue {
	/// Represents a "COMM" frame
	///
	/// Due to the amount of information needed, it is contained in a separate struct, [`LanguageFrame`]
	Comment(LanguageFrame),
	/// Represents a "USLT" frame
	///
	/// Due to the amount of information needed, it is contained in a separate struct, [`LanguageFrame`]
	UnSyncText(LanguageFrame),
	/// Represents a "T..." (excluding TXXX) frame
	///
	/// NOTE: Text frame names **must** be unique
	///
	Text {
		encoding: TextEncoding,
		value: String,
	},
	/// Represents a "TXXX" frame
	///
	/// Due to the amount of information needed, it is contained in a separate struct, [`EncodedTextFrame`]
	UserText(EncodedTextFrame),
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
	/// Due to the amount of information needed, it is contained in a separate struct, [`EncodedTextFrame`]
	UserURL(EncodedTextFrame),
	/// Represents an "APIC" or "PIC" frame
	Picture(Picture),
	/// Binary data
	///
	/// NOTES:
	///
	/// * This is used for "GEOB" and "SYLT" frames, see
	/// [`GeneralEncapsulatedObject::parse`](crate::id3::v2::GeneralEncapsulatedObject::parse) and [`SynchronizedText::parse`](crate::id3::v2::SynchronizedText::parse) respectively
	/// * This is used for **all** frames with an ID of [`FrameID::Outdated`]
	/// * This is used for unknown frames
	Binary(Vec<u8>),
}

impl From<ItemValue> for FrameValue {
	fn from(input: ItemValue) -> Self {
		match input {
			ItemValue::Text(text) => FrameValue::Text {
				encoding: TextEncoding::UTF8,
				value: text,
			},
			ItemValue::Locator(locator) => FrameValue::URL(locator),
			ItemValue::Binary(binary) => FrameValue::Binary(binary),
		}
	}
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[allow(clippy::struct_excessive_bools)]
/// Various flags to describe the content of an item
pub struct FrameFlags {
	/// Preserve frame on tag edit
	pub tag_alter_preservation: bool,
	/// Preserve frame on file edit
	pub file_alter_preservation: bool,
	/// Item cannot be written to
	pub read_only: bool,
	/// Frame belongs in a group
	///
	/// In addition to setting this flag, a group identifier byte must be added.
	/// All frames with the same group identifier byte belong to the same group.
	pub grouping_identity: (bool, u8),
	/// Frame is zlib compressed
	///
	/// It is **required** `data_length_indicator` be set if this is set.
	pub compression: bool,
	/// Frame is encrypted
	///
	/// NOTE: Since the encryption method is unknown, lofty cannot do anything with these frames
	///
	/// In addition to setting this flag, an encryption method symbol must be added.
	/// The method symbol **must** be > 0x80.
	pub encryption: (bool, u8),
	/// Frame is unsynchronised
	///
	/// In short, this makes all "0xFF 0x00" combinations into "0xFF 0x00 0x00" to avoid confusion
	/// with the MPEG frame header, which is often identified by its "frame sync" (11 set bits).
	/// It is preferred an ID3v2 tag is either *completely* unsynchronised or not unsynchronised at all.
	pub unsynchronisation: bool,
	/// Frame has a data length indicator
	///
	/// The data length indicator is the size of the frame if the flags were all zeroed out.
	/// This is usually used in combination with `compression` and `encryption` (depending on encryption method).
	///
	/// If using encryption, the final size must be added. It will be ignored if using compression.
	pub data_length_indicator: (bool, u32),
}

pub(crate) struct FrameRef<'a> {
	pub id: &'a str,
	pub value: FrameValueRef<'a>,
	pub flags: FrameFlags,
}

impl<'a> Frame {
	pub(crate) fn as_opt_ref(&'a self) -> Option<FrameRef<'a>> {
		if let FrameID::Valid(id) = &self.id {
			Some(FrameRef {
				id,
				value: (&self.value).into(),
				flags: self.flags,
			})
		} else {
			None
		}
	}
}

impl<'a> TryFrom<&'a TagItem> for FrameRef<'a> {
	type Error = LoftyError;

	fn try_from(value: &'a TagItem) -> std::prelude::rust_2015::Result<Self, Self::Error> {
		let id = match value.key() {
			ItemKey::Unknown(unknown)
				if unknown.len() == 4
					&& unknown.is_ascii()
					&& unknown.chars().all(|c| c.is_ascii_uppercase()) =>
			{
				Ok(unknown.as_str())
			}
			k => k.map_key(&TagType::Id3v2, false).ok_or(LoftyError::Id3v2(
				"ItemKey does not meet the requirements to be a FrameID",
			)),
		}?;

		Ok(FrameRef {
			id,
			value: Into::<FrameValueRef<'a>>::into(value.value()),
			flags: FrameFlags::default(),
		})
	}
}

pub(crate) enum FrameValueRef<'a> {
	Comment(&'a LanguageFrame),
	UnSyncText(&'a LanguageFrame),
	Text {
		encoding: TextEncoding,
		value: &'a str,
	},
	UserText(&'a EncodedTextFrame),
	URL(&'a str),
	UserURL(&'a EncodedTextFrame),
	Picture(&'a Picture),
	Binary(&'a [u8]),
}

impl<'a> Into<FrameValueRef<'a>> for &'a FrameValue {
	fn into(self) -> FrameValueRef<'a> {
		match self {
			FrameValue::Comment(lf) => FrameValueRef::Comment(lf),
			FrameValue::UnSyncText(lf) => FrameValueRef::UnSyncText(lf),
			FrameValue::Text { encoding, value } => FrameValueRef::Text {
				encoding: *encoding,
				value: value.as_str(),
			},
			FrameValue::UserText(etf) => FrameValueRef::UserText(etf),
			FrameValue::URL(url) => FrameValueRef::URL(url.as_str()),
			FrameValue::UserURL(etf) => FrameValueRef::UserURL(etf),
			FrameValue::Picture(pic) => FrameValueRef::Picture(pic),
			FrameValue::Binary(bin) => FrameValueRef::Binary(bin.as_slice()),
		}
	}
}

impl<'a> Into<FrameValueRef<'a>> for &'a ItemValue {
	fn into(self) -> FrameValueRef<'a> {
		match self {
			ItemValue::Text(text) => FrameValueRef::Text {
				encoding: TextEncoding::UTF8,
				value: text.as_str(),
			},
			ItemValue::Locator(locator) => FrameValueRef::URL(locator.as_str()),
			ItemValue::Binary(binary) => FrameValueRef::Binary(binary.as_slice()),
		}
	}
}
