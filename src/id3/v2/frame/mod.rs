pub(super) mod content;
mod header;
pub(super) mod id;
pub(super) mod read;

use super::items::{
	AttachedPictureFrame, CommentFrame, ExtendedTextFrame, ExtendedUrlFrame, Popularimeter,
	TextInformationFrame, UniqueFileIdentifierFrame, UnsynchronizedTextFrame, UrlLinkFrame,
};
use super::util::upgrade::{upgrade_v2, upgrade_v3};
use super::ID3v2Version;
use crate::error::{ErrorKind, ID3v2Error, ID3v2ErrorKind, LoftyError, Result};
use crate::tag::item::{ItemKey, ItemValue, TagItem};
use crate::tag::TagType;
use crate::util::text::TextEncoding;
use id::FrameID;

use std::borrow::Cow;
use std::convert::{TryFrom, TryInto};
use std::hash::{Hash, Hasher};

pub(super) const MUSICBRAINZ_UFID_OWNER: &str = "http://musicbrainz.org";

/// Empty content descriptor in text frame
///
/// Unspecific [`CommentFrame`]s, [`UnsynchronizedTextFrame`]s, and [`ExtendedTextFrame`] frames
/// are supposed to have an empty content descriptor. Only those
/// are currently supported as [`TagItem`]s to avoid ambiguities
/// and to prevent inconsistencies when writing them.
pub(super) const EMPTY_CONTENT_DESCRIPTOR: String = String::new();

/// Unknown language-aware text frame
///
/// <https://mutagen-specs.readthedocs.io/en/latest/id3/id3v2.4.0-structure.html>
///
/// > The three byte language field, present in several frames, is used to describe
/// > the language of the frame’s content, according to ISO-639-2 [ISO-639-2].
/// > The language should be represented in lower case. If the language is not known
/// > the string “XXX” should be used.
pub(super) const UNKNOWN_LANGUAGE: [u8; 3] = *b"XXX";

// TODO: Messy module, rough conversions

/// Represents an `ID3v2` frame
///
/// ## Outdated Frames
///
/// ### ID3v2.2
///
/// `ID3v2.2` frame IDs are 3 characters. When reading these tags, [`upgrade_v2`] is used, which has a list of all of the common IDs
/// that have a mapping to `ID3v2.4`. Any ID that fails to be converted will be stored as [`FrameID::Outdated`], and it must be manually
/// upgraded before it can be written. **Lofty** will not write `ID3v2.2` tags.
///
/// ### ID3v2.3
///
/// `ID3v2.3`, unlike `ID3v2.2`, stores frame IDs in 4 characters like `ID3v2.4`. There are some IDs that need upgrading (See [`upgrade_v3`]),
/// but anything that fails to be upgraded **will not** be stored as [`FrameID::Outdated`], as it is likely not an issue to write.
#[derive(Clone, Debug, Eq)]
pub struct Frame<'a> {
	pub(super) id: FrameID<'a>,
	pub(super) value: FrameValue,
	pub(super) flags: FrameFlags,
}

impl<'a> PartialEq for Frame<'a> {
	fn eq(&self, other: &Self) -> bool {
		match self.value {
			FrameValue::Text { .. } => self.id == other.id,
			_ => self.id == other.id && self.value == other.value,
		}
	}
}

impl<'a> Hash for Frame<'a> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		match self.value {
			FrameValue::Text { .. } => self.id.hash(state),
			_ => {
				self.id.hash(state);
				self.content().hash(state);
			},
		}
	}
}

impl<'a> Frame<'a> {
	/// Create a new frame
	///
	/// NOTE: This will accept both `ID3v2.2` and `ID3v2.3/4` frame IDs
	///
	/// # Errors
	///
	/// * `id` is less than 3 or greater than 4 bytes
	/// * `id` contains non-ascii characters
	pub fn new<I>(id: I, value: FrameValue, flags: FrameFlags) -> Result<Self>
	where
		I: Into<Cow<'a, str>>,
	{
		Self::new_cow(id.into(), value, flags)
	}

	// Split from generic, public method to avoid code bloat by monomorphization.
	fn new_cow(id: Cow<'a, str>, value: FrameValue, flags: FrameFlags) -> Result<Self> {
		let id_upgraded = match id.len() {
			// An ID with a length of 4 could be either V3 or V4.
			4 => match upgrade_v3(&id) {
				None => id,
				Some(upgraded) => Cow::Borrowed(upgraded),
			},
			3 => match upgrade_v2(&id) {
				None => id,
				Some(upgraded) => Cow::Borrowed(upgraded),
			},
			_ => return Err(ID3v2Error::new(ID3v2ErrorKind::BadFrameID).into()),
		};

		let id = FrameID::new_cow(id_upgraded)?;

		Ok(Self { id, value, flags })
	}

	/// Extract the string from the [`FrameID`]
	pub fn id_str(&self) -> &str {
		self.id.as_str()
	}

	/// Returns the frame's content
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

	// Used internally, has no correctness checks
	pub(crate) fn text(id: Cow<'a, str>, content: String) -> Self {
		Self {
			id: FrameID::Valid(id),
			value: FrameValue::Text(TextInformationFrame {
				encoding: TextEncoding::UTF8,
				value: content,
			}),
			flags: FrameFlags::default(),
		}
	}
}

/// The value of an `ID3v2` frame
#[non_exhaustive]
#[derive(PartialEq, Clone, Debug, Eq, Hash)]
pub enum FrameValue {
	/// Represents a "COMM" frame
	Comment(CommentFrame),
	/// Represents a "USLT" frame
	UnSyncText(UnsynchronizedTextFrame),
	/// Represents a "T..." (excluding TXXX) frame
	Text(TextInformationFrame),
	/// Represents a "TXXX" frame
	UserText(ExtendedTextFrame),
	/// Represents a "W..." (excluding WXXX) frame
	URL(UrlLinkFrame),
	/// Represents a "WXXX" frame
	UserURL(ExtendedUrlFrame),
	/// Represents an "APIC" or "PIC" frame
	Picture(AttachedPictureFrame),
	/// Represents a "POPM" frame
	Popularimeter(Popularimeter),
	/// Binary data
	///
	/// NOTES:
	///
	/// * This is used for "GEOB" and "SYLT" frames, see
	/// [`GeneralEncapsulatedObject::parse`](crate::id3::v2::GeneralEncapsulatedObject::parse) and [`SynchronizedText::parse`](crate::id3::v2::SynchronizedText::parse) respectively
	/// * This is used for **all** frames with an ID of [`FrameID::Outdated`]
	/// * This is used for unknown frames
	Binary(Vec<u8>),
	/// Unique file identifier
	UniqueFileIdentifier(UniqueFileIdentifierFrame),
}

impl TryFrom<ItemValue> for FrameValue {
	type Error = LoftyError;

	fn try_from(input: ItemValue) -> std::result::Result<FrameValue, Self::Error> {
		match input {
			ItemValue::Text(text) => Ok(FrameValue::Text(TextInformationFrame {
				encoding: TextEncoding::UTF8,
				value: text,
			})),
			ItemValue::Locator(locator) => {
				if TextEncoding::verify_latin1(&locator) {
					Ok(FrameValue::URL(UrlLinkFrame(locator)))
				} else {
					Err(LoftyError::new(ErrorKind::TextDecode(
						"ID3v2 URL frames must be Latin-1",
					)))
				}
			},
			ItemValue::Binary(binary) => Ok(FrameValue::Binary(binary)),
		}
	}
}

impl From<CommentFrame> for FrameValue {
	fn from(value: CommentFrame) -> Self {
		Self::Comment(value)
	}
}

impl From<UnsynchronizedTextFrame> for FrameValue {
	fn from(value: UnsynchronizedTextFrame) -> Self {
		Self::UnSyncText(value)
	}
}

impl From<TextInformationFrame> for FrameValue {
	fn from(value: TextInformationFrame) -> Self {
		Self::Text(value)
	}
}

impl From<ExtendedTextFrame> for FrameValue {
	fn from(value: ExtendedTextFrame) -> Self {
		Self::UserText(value)
	}
}

impl From<UrlLinkFrame> for FrameValue {
	fn from(value: UrlLinkFrame) -> Self {
		Self::URL(value)
	}
}

impl From<ExtendedUrlFrame> for FrameValue {
	fn from(value: ExtendedUrlFrame) -> Self {
		Self::UserURL(value)
	}
}

impl From<AttachedPictureFrame> for FrameValue {
	fn from(value: AttachedPictureFrame) -> Self {
		Self::Picture(value)
	}
}

impl From<Popularimeter> for FrameValue {
	fn from(value: Popularimeter) -> Self {
		Self::Popularimeter(value)
	}
}

impl From<UniqueFileIdentifierFrame> for FrameValue {
	fn from(value: UniqueFileIdentifierFrame) -> Self {
		Self::UniqueFileIdentifier(value)
	}
}

impl FrameValue {
	pub(super) fn as_bytes(&self) -> Result<Vec<u8>> {
		Ok(match self {
			FrameValue::Comment(comment) => comment.as_bytes()?,
			FrameValue::UnSyncText(lf) => lf.as_bytes()?,
			FrameValue::Text(tif) => tif.as_bytes(),
			FrameValue::UserText(content) => content.as_bytes(),
			FrameValue::UserURL(content) => content.as_bytes(),
			FrameValue::URL(link) => link.as_bytes(),
			FrameValue::Picture(attached_picture) => attached_picture.as_bytes(ID3v2Version::V4)?,
			FrameValue::Popularimeter(popularimeter) => popularimeter.as_bytes(),
			FrameValue::Binary(binary) => binary.clone(),
			FrameValue::UniqueFileIdentifier(frame) => frame.as_bytes(),
		})
	}
}

/// Various flags to describe the content of an item
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct FrameFlags {
	/// Preserve frame on tag edit
	pub tag_alter_preservation: bool,
	/// Preserve frame on file edit
	pub file_alter_preservation: bool,
	/// Item cannot be written to
	pub read_only: bool,
	/// The group identifier the frame belongs to
	///
	/// All frames with the same group identifier byte belong to the same group.
	pub grouping_identity: Option<u8>,
	/// Frame is zlib compressed
	///
	/// It is **required** `data_length_indicator` be set if this is set.
	pub compression: bool,
	/// Frame encryption method symbol
	///
	/// NOTE: Since the encryption method is unknown, lofty cannot do anything with these frames
	///
	/// The encryption method symbol **must** be > 0x80.
	pub encryption: Option<u8>,
	/// Frame is unsynchronised
	///
	/// In short, this makes all "0xFF X (X >= 0xE0)" combinations into "0xFF 0x00 X" to avoid confusion
	/// with the MPEG frame header, which is often identified by its "frame sync" (11 set bits).
	/// It is preferred an ID3v2 tag is either *completely* unsynchronised or not unsynchronised at all.
	///
	/// NOTE: While unsynchronized data is read, for the sake of simplicity, this flag has no effect when
	/// writing. There isn't much reason to write unsynchronized data.
	pub unsynchronisation: bool, /* TODO: Maybe? This doesn't seem very useful, and it is wasted effort if one forgets to make this false when writing. */
	/// Frame has a data length indicator
	///
	/// The data length indicator is the size of the frame if the flags were all zeroed out.
	/// This is usually used in combination with `compression` and `encryption` (depending on encryption method).
	///
	/// If using `encryption`, the final size must be added.
	pub data_length_indicator: Option<u32>,
}

impl From<TagItem> for Option<Frame<'static>> {
	fn from(input: TagItem) -> Self {
		let frame_id;
		let value;
		match input.key().try_into().map(FrameID::into_owned) {
			Ok(id) => {
				value = match (&id, input.item_value) {
					(FrameID::Valid(ref s), ItemValue::Text(text)) if s == "COMM" => {
						FrameValue::Comment(CommentFrame {
							encoding: TextEncoding::UTF8,
							language: UNKNOWN_LANGUAGE,
							description: EMPTY_CONTENT_DESCRIPTOR,
							content: text,
						})
					},
					(FrameID::Valid(ref s), ItemValue::Text(text)) if s == "USLT" => {
						FrameValue::UnSyncText(UnsynchronizedTextFrame {
							encoding: TextEncoding::UTF8,
							language: UNKNOWN_LANGUAGE,
							description: EMPTY_CONTENT_DESCRIPTOR,
							content: text,
						})
					},
					(FrameID::Valid(ref s), ItemValue::Locator(text) | ItemValue::Text(text))
						if s == "WXXX" =>
					{
						FrameValue::UserURL(ExtendedUrlFrame {
							encoding: TextEncoding::UTF8,
							description: EMPTY_CONTENT_DESCRIPTOR,
							content: text,
						})
					},
					(FrameID::Valid(ref s), ItemValue::Text(text)) if s == "TXXX" => {
						FrameValue::UserText(ExtendedTextFrame {
							encoding: TextEncoding::UTF8,
							description: EMPTY_CONTENT_DESCRIPTOR,
							content: text,
						})
					},
					(FrameID::Valid(ref s), ItemValue::Binary(text)) if s == "POPM" => {
						FrameValue::Popularimeter(Popularimeter::parse(&text).ok()?)
					},
					(_, item_value) => {
						let Ok(value) = item_value.try_into() else {
							return None;
						};

						value
					},
				};

				frame_id = id;
			},
			Err(_) => match input.item_key.map_key(TagType::ID3v2, true) {
				Some(desc) => match input.item_value {
					ItemValue::Text(text) => {
						frame_id = FrameID::Valid(Cow::Borrowed("TXXX"));
						value = FrameValue::UserText(ExtendedTextFrame {
							encoding: TextEncoding::UTF8,
							description: String::from(desc),
							content: text,
						})
					},
					ItemValue::Locator(locator) => {
						frame_id = FrameID::Valid(Cow::Borrowed("WXXX"));
						value = FrameValue::UserURL(ExtendedUrlFrame {
							encoding: TextEncoding::UTF8,
							description: String::from(desc),
							content: locator,
						})
					},
					ItemValue::Binary(_) => return None,
				},
				None => match (input.item_key, input.item_value) {
					(ItemKey::MusicBrainzRecordingId, ItemValue::Text(recording_id)) => {
						if !recording_id.is_ascii() {
							return None;
						}
						let frame = UniqueFileIdentifierFrame {
							owner: MUSICBRAINZ_UFID_OWNER.to_owned(),
							identifier: recording_id.into_bytes(),
						};
						frame_id = FrameID::Valid(Cow::Borrowed("UFID"));
						value = FrameValue::UniqueFileIdentifier(frame);
					},
					_ => {
						return None;
					},
				},
			},
		}

		Some(Frame {
			id: frame_id,
			value,
			flags: FrameFlags::default(),
		})
	}
}

#[derive(Clone)]
pub(crate) struct FrameRef<'a> {
	pub id: FrameID<'a>,
	pub value: Cow<'a, FrameValue>,
	pub flags: FrameFlags,
}

impl<'a> Frame<'a> {
	pub(crate) fn as_opt_ref(&'a self) -> Option<FrameRef<'a>> {
		if let FrameID::Valid(id) = &self.id {
			Some(FrameRef {
				id: FrameID::Valid(Cow::Borrowed(id)),
				value: Cow::Borrowed(self.content()),
				flags: self.flags,
			})
		} else {
			None
		}
	}
}

impl<'a> TryFrom<&'a TagItem> for FrameRef<'a> {
	type Error = LoftyError;

	fn try_from(tag_item: &'a TagItem) -> std::result::Result<Self, Self::Error> {
		let id: Result<FrameID<'a>> = tag_item.key().try_into();
		let frame_id: FrameID<'a>;
		let value: FrameValue;
		match id {
			Ok(id) => {
				let id_str = id.as_str();

				value = match (id_str, tag_item.value()) {
					("COMM", ItemValue::Text(text)) => FrameValue::Comment(CommentFrame {
						encoding: TextEncoding::UTF8,
						language: UNKNOWN_LANGUAGE,
						description: EMPTY_CONTENT_DESCRIPTOR,
						content: text.clone(),
					}),
					("USLT", ItemValue::Text(text)) => {
						FrameValue::UnSyncText(UnsynchronizedTextFrame {
							encoding: TextEncoding::UTF8,
							language: UNKNOWN_LANGUAGE,
							description: EMPTY_CONTENT_DESCRIPTOR,
							content: text.clone(),
						})
					},
					("WXXX", ItemValue::Locator(text) | ItemValue::Text(text)) => {
						FrameValue::UserURL(ExtendedUrlFrame {
							encoding: TextEncoding::UTF8,
							description: EMPTY_CONTENT_DESCRIPTOR,
							content: text.clone(),
						})
					},
					(locator_id, ItemValue::Locator(text)) if locator_id.len() > 4 => {
						FrameValue::UserURL(ExtendedUrlFrame {
							encoding: TextEncoding::UTF8,
							description: EMPTY_CONTENT_DESCRIPTOR,
							content: text.clone(),
						})
					},
					("TXXX", ItemValue::Text(text)) => FrameValue::UserText(ExtendedTextFrame {
						encoding: TextEncoding::UTF8,
						description: EMPTY_CONTENT_DESCRIPTOR,
						content: text.clone(),
					}),
					(text_id, ItemValue::Text(text)) if text_id.len() > 4 => {
						FrameValue::UserText(ExtendedTextFrame {
							encoding: TextEncoding::UTF8,
							description: EMPTY_CONTENT_DESCRIPTOR,
							content: text.clone(),
						})
					},
					("POPM", ItemValue::Binary(contents)) => {
						FrameValue::Popularimeter(Popularimeter::parse(contents)?)
					},
					(_, value) => value.try_into()?,
				};

				frame_id = id;
			},
			Err(_) => {
				let Some(desc) = tag_item.key().map_key(TagType::ID3v2, true) else {
					return Err(ID3v2Error::new(ID3v2ErrorKind::BadFrameID).into());
				};

				match tag_item.value() {
					ItemValue::Text(text) => {
						frame_id = FrameID::Valid(Cow::Borrowed("TXXX"));
						value = FrameValue::UserText(ExtendedTextFrame {
							encoding: TextEncoding::UTF8,
							description: String::from(desc),
							content: text.clone(),
						})
					},
					ItemValue::Locator(locator) => {
						frame_id = FrameID::Valid(Cow::Borrowed("WXXX"));
						value = FrameValue::UserURL(ExtendedUrlFrame {
							encoding: TextEncoding::UTF8,
							description: String::from(desc),
							content: locator.clone(),
						})
					},
					_ => return Err(ID3v2Error::new(ID3v2ErrorKind::BadFrameID).into()),
				}
			},
		}

		Ok(FrameRef {
			id: frame_id,
			value: Cow::Owned(value),
			flags: FrameFlags::default(),
		})
	}
}

impl<'a> TryInto<FrameValue> for &'a ItemValue {
	type Error = LoftyError;

	fn try_into(self) -> std::result::Result<FrameValue, Self::Error> {
		match self {
			ItemValue::Text(text) => Ok(FrameValue::Text(TextInformationFrame {
				encoding: TextEncoding::UTF8,
				value: text.clone(),
			})),
			ItemValue::Locator(locator) => {
				if TextEncoding::verify_latin1(locator) {
					Ok(FrameValue::URL(UrlLinkFrame(locator.clone())))
				} else {
					Err(LoftyError::new(ErrorKind::TextDecode(
						"ID3v2 URL frames must be Latin-1",
					)))
				}
			},
			ItemValue::Binary(binary) => Ok(FrameValue::Binary(binary.clone())),
		}
	}
}
