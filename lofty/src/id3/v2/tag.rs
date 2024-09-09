#[cfg(test)]
mod tests;

use super::frame::{Frame, EMPTY_CONTENT_DESCRIPTOR};
use super::header::{Id3v2TagFlags, Id3v2Version};
use crate::config::{global_options, WriteOptions};
use crate::error::{LoftyError, Result};
use crate::id3::v1::GENRES;
use crate::id3::v2::frame::{FrameRef, MUSICBRAINZ_UFID_OWNER};
use crate::id3::v2::items::{
	AttachedPictureFrame, CommentFrame, ExtendedTextFrame, ExtendedUrlFrame, TextInformationFrame,
	UniqueFileIdentifierFrame, UnsynchronizedTextFrame, UrlLinkFrame,
};
use crate::id3::v2::util::mappings::TIPL_MAPPINGS;
use crate::id3::v2::util::pairs::{
	format_number_pair, set_number, NUMBER_PAIR_KEYS, NUMBER_PAIR_SEPARATOR,
};
use crate::id3::v2::{BinaryFrame, FrameHeader, FrameId, KeyValueFrame, TimestampFrame};
use crate::mp4::AdvisoryRating;
use crate::picture::{Picture, PictureType, TOMBSTONE_PICTURE};
use crate::tag::companion_tag::CompanionTag;
use crate::tag::items::{Lang, Timestamp, UNKNOWN_LANGUAGE};
use crate::tag::{Accessor, ItemKey, ItemValue, MergeTag, SplitTag, Tag, TagExt, TagItem, TagType};
use crate::util::flag_item;
use crate::util::io::{FileLike, Length, Truncate};
use crate::util::text::{decode_text, TextDecodeOptions, TextEncoding};

use std::borrow::Cow;
use std::io::{Cursor, Write};
use std::iter::Peekable;
use std::ops::Deref;
use std::str::FromStr;

use lofty_attr::tag;

const INVOLVED_PEOPLE_LIST_ID: &str = "TIPL";

const V4_MULTI_VALUE_SEPARATOR: char = '\0';

// Used exclusively for `Accessor` convenience methods
fn remove_separators_from_frame_text(value: &str, version: Id3v2Version) -> Cow<'_, str> {
	if !value.contains(V4_MULTI_VALUE_SEPARATOR) || version != Id3v2Version::V4 {
		return Cow::Borrowed(value);
	}

	return Cow::Owned(value.replace(V4_MULTI_VALUE_SEPARATOR, "/"));
}

macro_rules! impl_accessor {
	($($name:ident => $id:literal;)+) => {
		paste::paste! {
			$(
				fn $name(&self) -> Option<Cow<'_, str>> {
					if let Some(value) = self.get_text(&[<$name:upper _ID>]) {
						return Some(remove_separators_from_frame_text(value, self.original_version));
					}

					None
				}

				fn [<set_ $name>](&mut self, value: String) {
					self.insert(new_text_frame(
						[<$name:upper _ID>],
						value,
					));
				}

				fn [<remove_ $name>](&mut self) {
					let _ = self.remove(&[<$name:upper _ID>]);
				}
			)+
		}
	}
}

/// ## [`Accessor`] Methods
///
/// As ID3v2.4 allows for multiple values to exist in a single frame, the raw strings, as provided by [`Id3v2Tag::get_text`]
/// may contain null separators.
///
/// In the [`Accessor`] methods, these values have the separators (`\0`) replaced with `"/"` for convenience.
///
/// ## Conversions
///
/// ⚠ **Warnings** ⚠
///
/// ### From `Tag`
///
/// When converting from a [`Tag`] to an `Id3v2Tag`, some frames may need editing.
///
/// * [`ItemKey::Comment`] and [`ItemKey::Lyrics`] - Unlike a normal text frame, these require a language. See [`CommentFrame`] and [`UnsynchronizedTextFrame`] respectively.
/// An attempt is made to create this information, but it may be incorrect.
///    * `language` - Unknown and set to "XXX"
///    * `description` - Left empty, which is invalid if there are more than one of these frames. These frames can only be identified
///    by their descriptions, and as such they are expected to be unique for each.
/// * [`ItemKey::Unknown("WXXX" | "TXXX")`](ItemKey::Unknown) - These frames are also identified by their descriptions.
///
/// ### To `Tag`
///
/// * TXXX/WXXX - These frames will be stored as an [`ItemKey`] by their description. Some variants exist for these descriptions, such as the one for `ReplayGain`,
/// otherwise [`ItemKey::Unknown`] will be used.
/// * Frames that require a language (COMM/USLT) - With ID3v2 being the only format that allows for language-specific items, this information is not retained.
/// * POPM - These frames will be stored as a raw [`ItemValue::Binary`] value under the [`ItemKey::Popularimeter`] key.
///
/// ## Special Frames
///
/// ID3v2 has `GEOB` and `SYLT` frames, which are not parsed by default, instead storing them as [`FrameType::Binary`].
/// They can easily be parsed with [`GeneralEncapsulatedObject::parse`](crate::id3::v2::GeneralEncapsulatedObject::parse)
/// and [`SynchronizedText::parse`](crate::id3::v2::SynchronizedTextFrame::parse) respectively, and converted back to binary with
/// [`GeneralEncapsulatedObject::as_bytes`](crate::id3::v2::GeneralEncapsulatedObject::as_bytes) and
/// [`SynchronizedText::as_bytes`](crate::id3::v2::SynchronizedTextFrame::as_bytes) for writing.
#[derive(PartialEq, Eq, Debug, Clone)]
#[tag(
	description = "An `ID3v2` tag",
	supported_formats(Aac, Aiff, Mpeg, Wav, read_only(Ape, Flac, Mpc))
)]
pub struct Id3v2Tag {
	flags: Id3v2TagFlags,
	pub(super) original_version: Id3v2Version,
	pub(crate) frames: Vec<Frame<'static>>,
}

impl IntoIterator for Id3v2Tag {
	type Item = Frame<'static>;
	type IntoIter = std::vec::IntoIter<Self::Item>;

	fn into_iter(self) -> Self::IntoIter {
		self.frames.into_iter()
	}
}

impl<'a> IntoIterator for &'a Id3v2Tag {
	type Item = &'a Frame<'static>;
	type IntoIter = std::slice::Iter<'a, Frame<'static>>;

	fn into_iter(self) -> Self::IntoIter {
		self.frames.iter()
	}
}

impl Default for Id3v2Tag {
	fn default() -> Self {
		Self {
			flags: Id3v2TagFlags::default(),
			original_version: Id3v2Version::V4,
			frames: Vec::new(),
		}
	}
}

impl Id3v2Tag {
	/// Create a new empty `ID3v2Tag`
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::id3::v2::Id3v2Tag;
	/// use lofty::tag::TagExt;
	///
	/// let id3v2_tag = Id3v2Tag::new();
	/// assert!(id3v2_tag.is_empty());
	/// ```
	pub fn new() -> Self {
		Self::default()
	}

	/// Returns the [`Id3v2TagFlags`]
	pub fn flags(&self) -> &Id3v2TagFlags {
		&self.flags
	}

	/// Restrict the tag's flags
	pub fn set_flags(&mut self, flags: Id3v2TagFlags) {
		self.flags = flags
	}

	/// The original version of the tag
	///
	/// This is here, since the tag is upgraded to `ID3v2.4`, but a `v2.2` or `v2.3`
	/// tag may have been read.
	pub fn original_version(&self) -> Id3v2Version {
		self.original_version
	}
}

impl Id3v2Tag {
	/// Gets a [`Frame`] from an id
	pub fn get(&self, id: &FrameId<'_>) -> Option<&Frame<'static>> {
		self.frames.iter().find(|f| f.id() == id)
	}

	/// Gets the text for a frame
	///
	/// NOTE: If the tag is [`Id3v2Version::V4`], there could be multiple values separated by null characters (`'\0'`).
	///       Use [`Id3v2Tag::get_texts`] to conveniently split all of the values.
	///
	/// NOTE: This will not work for `TXXX` frames, use [`Id3v2Tag::get_user_text`] for that.
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::id3::v2::{FrameId, Id3v2Tag};
	/// use lofty::tag::Accessor;
	/// use std::borrow::Cow;
	///
	/// const TITLE_ID: FrameId<'_> = FrameId::Valid(Cow::Borrowed("TIT2"));
	///
	/// let mut tag = Id3v2Tag::new();
	///
	/// tag.set_title(String::from("Foo"));
	///
	/// let title = tag.get_text(&TITLE_ID);
	/// assert_eq!(title, Some("Foo"));
	///
	/// // Now we have a string with multiple values
	/// tag.set_title(String::from("Foo\0Bar"));
	///
	/// // Null separator is retained! This case is better handled by `get_texts`.
	/// let title = tag.get_text(&TITLE_ID);
	/// assert_eq!(title, Some("Foo\0Bar"));
	/// ```
	pub fn get_text(&self, id: &FrameId<'_>) -> Option<&str> {
		let frame = self.get(id);
		if let Some(Frame::Text(TextInformationFrame { value, .. })) = frame {
			return Some(value);
		}

		None
	}

	/// Gets all of the values for a text frame
	///
	/// NOTE: Multiple values are only supported in ID3v2.4, this will not be
	///       very useful for ID3v2.2/3 tags.
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::id3::v2::{FrameId, Id3v2Tag};
	/// use lofty::tag::Accessor;
	/// use std::borrow::Cow;
	///
	/// const TITLE_ID: FrameId<'_> = FrameId::Valid(Cow::Borrowed("TIT2"));
	///
	/// let mut tag = Id3v2Tag::new();
	///
	/// tag.set_title(String::from("Foo\0Bar"));
	///
	/// let mut titles = tag.get_texts(&TITLE_ID).expect("Should exist");
	///
	/// assert_eq!(titles.next(), Some("Foo"));
	/// assert_eq!(titles.next(), Some("Bar"));
	/// ```
	pub fn get_texts(&self, id: &FrameId<'_>) -> Option<impl Iterator<Item = &str>> {
		if let Some(Frame::Text(TextInformationFrame { value, .. })) = self.get(id) {
			return Some(value.split(V4_MULTI_VALUE_SEPARATOR));
		}

		None
	}

	/// Gets the text for a user-defined frame
	///
	/// NOTE: If the tag is [`Id3v2Version::V4`], there could be multiple values separated by null characters (`'\0'`).
	///       The caller is responsible for splitting these values as necessary.
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::id3::v2::Id3v2Tag;
	///
	/// let mut tag = Id3v2Tag::new();
	///
	/// // Add a new "TXXX" frame identified by "SOME_DESCRIPTION"
	/// let _ = tag.insert_user_text(String::from("SOME_DESCRIPTION"), String::from("Some value"));
	///
	/// // Now we can get the value back using the description
	/// let value = tag.get_user_text("SOME_DESCRIPTION");
	/// assert_eq!(value, Some("Some value"));
	/// ```
	pub fn get_user_text(&self, description: &str) -> Option<&str> {
		self.frames
			.iter()
			.filter(|frame| frame.id().as_str() == "TXXX")
			.find_map(|frame| match frame {
				Frame::UserText(ExtendedTextFrame {
					description: desc,
					content,
					..
				}) if desc == description => Some(content.as_str()),
				_ => None,
			})
	}

	/// Inserts a new user-defined text frame (`TXXX`)
	///
	/// NOTE: The encoding will be UTF-8
	///
	/// This will replace any TXXX frame with the same description, see [`Id3v2Tag::insert`].
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::id3::v2::Id3v2Tag;
	/// use lofty::tag::TagExt;
	///
	/// let mut tag = Id3v2Tag::new();
	///
	/// assert!(tag.is_empty());
	///
	/// // Add a new "TXXX" frame identified by "SOME_DESCRIPTION"
	/// let _ = tag.insert_user_text(String::from("SOME_DESCRIPTION"), String::from("Some value"));
	///
	/// // Now we can get the value back using `get_user_text`
	/// let value = tag.get_user_text("SOME_DESCRIPTION");
	/// assert_eq!(value, Some("Some value"));
	/// ```
	pub fn insert_user_text(
		&mut self,
		description: String,
		content: String,
	) -> Option<Frame<'static>> {
		self.insert(Frame::UserText(ExtendedTextFrame::new(
			TextEncoding::UTF8,
			description,
			content,
		)))
	}

	/// Inserts a [`Frame`]
	///
	/// This will replace any frame of the same id (**or description!** See [`ExtendedTextFrame`])
	pub fn insert(&mut self, frame: Frame<'static>) -> Option<Frame<'static>> {
		// Some frames can only appear once in a tag, handle them separately
		const ONE_PER_TAG: [&str; 11] = [
			"MCDI", "ETCO", "MLLT", "SYTC", "RVRB", "PCNT", "RBUF", "POSS", "OWNE", "SEEK", "ASPI",
		];

		if ONE_PER_TAG.contains(&frame.id_str()) {
			let ret = self.remove(frame.id()).next();
			self.frames.push(frame);
			return ret;
		}

		let replaced = self
			.frames
			.iter()
			.position(|f| f == &frame)
			.map(|pos| self.frames.remove(pos));

		self.frames.push(frame);
		replaced
	}

	/// Removes a user-defined text frame (`TXXX`) by its description
	///
	/// This will return the matching frame.
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::id3::v2::Id3v2Tag;
	/// use lofty::tag::TagExt;
	///
	/// let mut tag = Id3v2Tag::new();
	/// assert!(tag.is_empty());
	///
	/// // Add a new "TXXX" frame identified by "SOME_DESCRIPTION"
	/// let _ = tag.insert_user_text(String::from("SOME_DESCRIPTION"), String::from("Some value"));
	/// assert!(!tag.is_empty());
	///
	/// // Now we can remove it by its description
	/// let value = tag.remove_user_text("SOME_DESCRIPTION");
	/// assert!(tag.is_empty());
	/// ```
	pub fn remove_user_text(&mut self, description: &str) -> Option<Frame<'static>> {
		self.frames
			.iter()
			.position(|frame| {
				matches!(frame, Frame::UserText(ExtendedTextFrame {
                             description: desc, ..
                         }) if desc == description)
			})
			.map(|pos| self.frames.remove(pos))
	}

	/// Removes a [`Frame`] by id
	///
	/// This will remove any frames with the same ID. To remove `TXXX` frames by their descriptions,
	/// see [`Id3v2Tag::remove_user_text`].
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::id3::v2::{Frame, FrameFlags, FrameId, Id3v2Tag, TextInformationFrame};
	/// use lofty::tag::TagExt;
	/// use lofty::TextEncoding;
	/// use std::borrow::Cow;
	///
	/// const MOOD_FRAME_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("TMOO"));
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// let mut tag = Id3v2Tag::new();
	/// assert!(tag.is_empty());
	///
	/// // Add a new "TMOO" frame
	/// let tmoo_frame = Frame::Text(TextInformationFrame::new(
	/// 	MOOD_FRAME_ID,
	/// 	TextEncoding::Latin1,
	/// 	String::from("Classical"),
	/// ));
	///
	/// let _ = tag.insert(tmoo_frame.clone());
	/// assert!(!tag.is_empty());
	///
	/// // Now we can remove it by its ID
	/// let mut values = tag.remove(&MOOD_FRAME_ID);
	///
	/// // We got back exactly what we inserted
	/// assert_eq!(values.next(), Some(tmoo_frame));
	/// assert!(values.next().is_none());
	/// drop(values);
	///
	/// // The tag is now empty
	/// assert!(tag.is_empty());
	/// # Ok(()) }
	/// ```
	pub fn remove(&mut self, id: &FrameId<'_>) -> impl Iterator<Item = Frame<'static>> + '_ {
		// TODO: drain_filter
		let mut split_idx = 0_usize;

		for read_idx in 0..self.frames.len() {
			if self.frames[read_idx].id() == id {
				self.frames.swap(split_idx, read_idx);
				split_idx += 1;
			}
		}

		self.frames.drain(..split_idx)
	}

	fn take_first(&mut self, id: &FrameId<'_>) -> Option<Frame<'static>> {
		self.frames
			.iter()
			.position(|f| f.id() == id)
			.map(|pos| self.frames.remove(pos))
	}

	/// Retains [`Frame`]s by evaluating the predicate
	pub fn retain<P>(&mut self, predicate: P)
	where
		P: FnMut(&Frame<'_>) -> bool,
	{
		self.frames.retain(predicate)
	}

	/// Inserts a [`Picture`]
	///
	/// According to spec, there can only be one picture of type [`PictureType::Icon`] and [`PictureType::OtherIcon`].
	/// When attempting to insert these types, if another is found it will be removed and returned.
	pub fn insert_picture(&mut self, picture: Picture) -> Option<Frame<'static>> {
		let ret = if picture.pic_type == PictureType::Icon
			|| picture.pic_type == PictureType::OtherIcon
		{
			let mut pos = None;

			for (i, frame) in self.frames.iter().enumerate() {
				match frame {
					Frame::Picture(AttachedPictureFrame {
						picture: Picture { pic_type, .. },
						..
					}) if pic_type == &picture.pic_type => {
						pos = Some(i);
						break;
					},
					_ => {},
				}
			}

			pos.map(|p| self.frames.remove(p))
		} else {
			None
		};

		self.frames.push(new_picture_frame(picture));

		ret
	}

	/// Removes a certain [`PictureType`]
	pub fn remove_picture_type(&mut self, picture_type: PictureType) {
		self.frames.retain(|f| {
			!matches!(f, Frame::Picture(AttachedPictureFrame {
						picture: Picture {
							pic_type: p_ty,
							..
						}, ..
					}) if p_ty == &picture_type)
		})
	}

	/// Returns all `USLT` frames
	pub fn unsync_text(&self) -> impl Iterator<Item = &UnsynchronizedTextFrame<'_>> + Clone {
		self.frames.iter().filter_map(|f| match f {
			Frame::UnsynchronizedText(val) => Some(val),
			_ => None,
		})
	}

	/// Returns all `COMM` frames with an empty content descriptor
	pub fn comments(&self) -> impl Iterator<Item = &CommentFrame<'_>> {
		self.frames.iter().filter_map(|frame| {
			filter_comment_frame_by_description(frame, &EMPTY_CONTENT_DESCRIPTOR)
		})
	}

	fn split_num_pair(&self, id: &FrameId<'_>) -> (Option<u32>, Option<u32>) {
		if let Some(Frame::Text(TextInformationFrame { ref value, .. })) = self.get(id) {
			let mut split = value
				.split(&[V4_MULTI_VALUE_SEPARATOR, NUMBER_PAIR_SEPARATOR][..])
				.flat_map(str::parse::<u32>);
			return (split.next(), split.next());
		}

		(None, None)
	}

	fn insert_item(&mut self, item: TagItem) {
		match item.key() {
			ItemKey::TrackNumber => set_number(&item, |number| self.set_track(number)),
			ItemKey::TrackTotal => set_number(&item, |number| self.set_track_total(number)),
			ItemKey::DiscNumber => set_number(&item, |number| self.set_disk(number)),
			ItemKey::DiscTotal => set_number(&item, |number| self.set_disk_total(number)),
			_ => {
				if let Some(frame) = item.into() {
					if let Some(replaced) = self.insert(frame) {
						log::warn!("Replaced frame: {replaced:?}");
					}
				}
			},
		};
	}

	/// Returns all genres contained in a `TCON` frame.
	///
	/// This will translate any numeric genre IDs to their textual equivalent.
	/// ID3v2.4-style multi-value fields will be split as normal.
	pub fn genres(&self) -> Option<impl Iterator<Item = &str>> {
		if let Some(Frame::Text(TextInformationFrame { ref value, .. })) = self.get(&GENRE_ID) {
			return Some(GenresIter::new(value, false));
		}

		None
	}

	fn insert_number_pair(
		&mut self,
		id: FrameId<'static>,
		number: Option<u32>,
		total: Option<u32>,
	) {
		if let Some(content) = format_number_pair(number, total) {
			self.insert(Frame::text(id.into_inner(), content));
		} else {
			log::warn!("{id} is not set. number: {number:?}, total: {total:?}");
		}
	}
}

pub(crate) struct GenresIter<'a> {
	value: &'a str,
	pos: usize,
	preserve_indexes: bool,
}

impl<'a> GenresIter<'a> {
	pub fn new(value: &'a str, preserve_indexes: bool) -> GenresIter<'_> {
		GenresIter {
			value,
			pos: 0,
			preserve_indexes,
		}
	}
}

impl<'a> Iterator for GenresIter<'a> {
	type Item = &'a str;

	fn next(&mut self) -> Option<Self::Item> {
		if self.pos >= self.value.len() {
			return None;
		}

		let remainder = &self.value[self.pos..];

		if let Some(idx) = remainder.find(V4_MULTI_VALUE_SEPARATOR) {
			let start = self.pos;
			let end = self.pos + idx;
			self.pos = end + 1;
			return Some(parse_genre(&self.value[start..end], self.preserve_indexes));
		}

		if remainder.starts_with('(') && remainder.contains(')') {
			let start = self.pos + 1;
			let mut end = self.pos + remainder.find(')').unwrap();
			self.pos = end + 1;
			// handle bracketed refinement e.g. (55)((I think...)"
			if remainder.starts_with("((") {
				end += 1;
			}
			return Some(parse_genre(&self.value[start..end], self.preserve_indexes));
		}

		self.pos = self.value.len();
		Some(parse_genre(remainder, self.preserve_indexes))
	}
}

fn parse_genre(genre: &str, preserve_indexes: bool) -> &str {
	if genre.len() > 3 {
		return genre;
	}
	if let Ok(id) = genre.parse::<usize>() {
		if id < GENRES.len() && !preserve_indexes {
			GENRES[id]
		} else {
			genre
		}
	} else if genre == "RX" {
		"Remix"
	} else if genre == "CR" {
		"Cover"
	} else {
		genre
	}
}

fn filter_comment_frame_by_description<'a>(
	frame: &'a Frame<'_>,
	description: &str,
) -> Option<&'a CommentFrame<'a>> {
	match &frame {
		Frame::Comment(comment_frame) => {
			(comment_frame.description == description).then_some(comment_frame)
		},
		_ => None,
	}
}

fn filter_comment_frame_by_description_mut<'a, 'f: 'a>(
	frame: &'a mut Frame<'f>,
	description: &str,
) -> Option<&'a mut CommentFrame<'f>> {
	match frame {
		Frame::Comment(comment_frame) => {
			(comment_frame.description == description).then_some(comment_frame)
		},
		_ => None,
	}
}

pub(super) fn new_text_frame(id: FrameId<'_>, value: String) -> Frame<'_> {
	Frame::Text(TextInformationFrame::new(id, TextEncoding::UTF8, value))
}

pub(super) fn new_url_frame(id: FrameId<'_>, value: String) -> Frame<'_> {
	Frame::Url(UrlLinkFrame::new(id, value))
}

pub(super) fn new_user_text_frame(description: String, content: String) -> Frame<'static> {
	Frame::UserText(ExtendedTextFrame::new(
		TextEncoding::UTF8,
		description,
		content,
	))
}

pub(super) fn new_user_url_frame(description: String, content: String) -> Frame<'static> {
	Frame::UserUrl(ExtendedUrlFrame::new(
		TextEncoding::UTF8,
		description,
		content,
	))
}

pub(super) fn new_comment_frame(content: String) -> Frame<'static> {
	Frame::Comment(CommentFrame::new(
		TextEncoding::UTF8,
		UNKNOWN_LANGUAGE,
		EMPTY_CONTENT_DESCRIPTOR,
		content,
	))
}

pub(super) fn new_picture_frame(picture: Picture) -> Frame<'static> {
	Frame::Picture(AttachedPictureFrame::new(TextEncoding::UTF8, picture))
}

pub(super) fn new_key_value_frame(
	id: FrameId<'_>,
	key_value_pairs: Vec<(String, String)>,
) -> Frame<'_> {
	Frame::KeyValue(KeyValueFrame::new(id, TextEncoding::UTF8, key_value_pairs))
}

pub(super) fn new_timestamp_frame(id: FrameId<'_>, timestamp: Timestamp) -> Frame<'_> {
	Frame::Timestamp(TimestampFrame::new(id, TextEncoding::UTF8, timestamp))
}

pub(super) fn new_unsync_text_frame(content: String) -> Frame<'static> {
	Frame::UnsynchronizedText(UnsynchronizedTextFrame::new(
		TextEncoding::UTF8,
		UNKNOWN_LANGUAGE,
		EMPTY_CONTENT_DESCRIPTOR,
		content,
	))
}

pub(super) fn new_binary_frame(id: FrameId<'_>, data: Vec<u8>) -> Frame<'_> {
	Frame::Binary(BinaryFrame::new(id, data))
}

const TITLE_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("TIT2"));
const ARTIST_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("TPE1"));
const ALBUM_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("TALB"));
const GENRE_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("TCON"));
const TRACK_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("TRCK"));
const DISC_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("TPOS"));
const RECORDING_TIME_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("TDRC"));
pub(super) const ATTACHED_PICTURE_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("APIC"));

impl Accessor for Id3v2Tag {
	impl_accessor!(
		title  => "TIT2";
		artist => "TPE1";
		album  => "TALB";
	);

	fn track(&self) -> Option<u32> {
		self.split_num_pair(&TRACK_ID).0
	}

	fn set_track(&mut self, value: u32) {
		self.insert_number_pair(TRACK_ID, Some(value), self.track_total());
	}

	fn remove_track(&mut self) {
		let _ = self.remove(&TRACK_ID);
	}

	fn track_total(&self) -> Option<u32> {
		self.split_num_pair(&TRACK_ID).1
	}

	fn set_track_total(&mut self, value: u32) {
		self.insert_number_pair(TRACK_ID, self.track(), Some(value));
	}

	fn remove_track_total(&mut self) {
		let existing_track_number = self.track();
		let _ = self.remove(&TRACK_ID);

		if let Some(track) = existing_track_number {
			self.insert(Frame::text(Cow::Borrowed("TRCK"), track.to_string()));
		}
	}

	fn disk(&self) -> Option<u32> {
		self.split_num_pair(&DISC_ID).0
	}

	fn set_disk(&mut self, value: u32) {
		self.insert_number_pair(DISC_ID, Some(value), self.disk_total());
	}

	fn remove_disk(&mut self) {
		let _ = self.remove(&DISC_ID);
	}

	fn disk_total(&self) -> Option<u32> {
		self.split_num_pair(&DISC_ID).1
	}

	fn set_disk_total(&mut self, value: u32) {
		self.insert_number_pair(DISC_ID, self.disk(), Some(value));
	}

	fn remove_disk_total(&mut self) {
		let existing_track_number = self.track();
		let _ = self.remove(&DISC_ID);

		if let Some(track) = existing_track_number {
			self.insert(Frame::text(Cow::Borrowed("TPOS"), track.to_string()));
		}
	}

	fn genre(&self) -> Option<Cow<'_, str>> {
		let mut genres = self.genres()?.peekable();
		let first = genres.next()?;

		if genres.peek().is_none() {
			return Some(Cow::Borrowed(first));
		};

		let mut joined = String::from(first);
		for genre in genres {
			joined.push_str(" / ");
			joined.push_str(genre);
		}

		Some(Cow::Owned(joined))
	}

	fn set_genre(&mut self, value: String) {
		self.insert(new_text_frame(GENRE_ID, value));
	}

	fn remove_genre(&mut self) {
		let _ = self.remove(&GENRE_ID);
	}

	fn year(&self) -> Option<u32> {
		if let Some(Frame::Timestamp(TimestampFrame { timestamp, .. })) =
			self.get(&RECORDING_TIME_ID)
		{
			return Some(u32::from(timestamp.year));
		}

		None
	}

	fn set_year(&mut self, value: u32) {
		self.insert(Frame::text(Cow::Borrowed("TDRC"), value.to_string()));
	}

	fn remove_year(&mut self) {
		let _ = self.remove(&RECORDING_TIME_ID);
	}

	fn comment(&self) -> Option<Cow<'_, str>> {
		self.frames
			.iter()
			.find_map(|frame| filter_comment_frame_by_description(frame, &EMPTY_CONTENT_DESCRIPTOR))
			.map(|CommentFrame { content, .. }| Cow::Borrowed(content.as_str()))
	}

	fn set_comment(&mut self, value: String) {
		let mut value = Some(value);
		self.frames.retain_mut(|frame| {
			let Some(CommentFrame { content, .. }) =
				filter_comment_frame_by_description_mut(frame, &EMPTY_CONTENT_DESCRIPTOR)
			else {
				return true;
			};
			if let Some(value) = value.take() {
				// Replace value in first comment frame
				*content = value;
				true
			} else {
				// Remove all subsequent comment frames
				false
			}
		});
		if let Some(value) = value {
			self.frames.push(new_comment_frame(value));
		}
	}

	fn remove_comment(&mut self) {
		self.frames.retain(|frame| {
			filter_comment_frame_by_description(frame, &EMPTY_CONTENT_DESCRIPTOR).is_none()
		})
	}
}

impl TagExt for Id3v2Tag {
	type Err = LoftyError;
	type RefKey<'a> = &'a FrameId<'a>;

	#[inline]
	fn tag_type(&self) -> TagType {
		TagType::Id3v2
	}

	fn len(&self) -> usize {
		self.frames.len()
	}

	fn contains<'a>(&'a self, key: Self::RefKey<'a>) -> bool {
		self.frames.iter().any(|frame| frame.id() == key)
	}

	fn is_empty(&self) -> bool {
		self.frames.is_empty()
	}

	/// Writes the tag to a file
	///
	/// # Errors
	///
	/// * Attempting to write the tag to a format that does not support it
	/// * Attempting to write an encrypted frame without a valid method symbol or data length indicator
	/// * Attempting to write an invalid [`FrameId`]/[`Frame`] pairing
	fn save_to<F>(
		&self,
		file: &mut F,
		write_options: WriteOptions,
	) -> std::result::Result<(), Self::Err>
	where
		F: FileLike,
		LoftyError: From<<F as Truncate>::Error>,
		LoftyError: From<<F as Length>::Error>,
	{
		Id3v2TagRef {
			flags: self.flags,
			frames: self.frames.iter().filter_map(Frame::as_opt_ref).peekable(),
		}
		.write_to(file, write_options)
	}

	/// Dumps the tag to a writer
	///
	/// # Errors
	///
	/// * [`std::io::Error`]
	/// * [`ErrorKind::TooMuchData`](crate::error::ErrorKind::TooMuchData)
	fn dump_to<W: Write>(
		&self,
		writer: &mut W,
		write_options: WriteOptions,
	) -> std::result::Result<(), Self::Err> {
		Id3v2TagRef {
			flags: self.flags,
			frames: self.frames.iter().filter_map(Frame::as_opt_ref).peekable(),
		}
		.dump_to(writer, write_options)
	}

	fn clear(&mut self) {
		self.frames.clear();
	}
}

#[derive(Debug, Clone, Default)]
pub struct SplitTagRemainder(Id3v2Tag);

impl From<SplitTagRemainder> for Id3v2Tag {
	fn from(from: SplitTagRemainder) -> Self {
		from.0
	}
}

impl Deref for SplitTagRemainder {
	type Target = Id3v2Tag;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

fn handle_tag_split(tag: &mut Tag, frame: &mut Frame<'_>) -> bool {
	/// A frame we are able to split off into the tag
	const FRAME_CONSUMED: bool = false;
	/// A frame that must be held back
	const FRAME_RETAINED: bool = true;

	fn split_pair(
		content: &str,
		tag: &mut Tag,
		number_key: ItemKey,
		total_key: ItemKey,
	) -> Option<()> {
		fn parse_number(source: &str) -> Option<&str> {
			let number = source.trim();

			if number.is_empty() {
				return None;
			}

			if str::parse::<u32>(number).is_ok() {
				Some(number)
			} else {
				log::warn!("{number:?} could not be parsed as a number.");

				None
			}
		}

		let mut split = content.splitn(2, &[V4_MULTI_VALUE_SEPARATOR, NUMBER_PAIR_SEPARATOR][..]);

		let number = parse_number(split.next()?)?;
		let total = if let Some(total_source) = split.next() {
			Some(parse_number(total_source)?)
		} else {
			None
		};
		debug_assert!(split.next().is_none());

		debug_assert!(!number.is_empty());
		tag.items.push(TagItem::new(
			number_key,
			ItemValue::Text(number.to_string()),
		));
		if let Some(total) = total {
			debug_assert!(!total.is_empty());
			tag.items
				.push(TagItem::new(total_key, ItemValue::Text(total.to_string())))
		}

		Some(())
	}

	match frame {
		// The text pairs need some special treatment
		Frame::Text(TextInformationFrame {
			header: FrameHeader { id, .. },
			value: content,
			..
		}) if id.as_str() == "TRCK"
			&& split_pair(content, tag, ItemKey::TrackNumber, ItemKey::TrackTotal).is_some() =>
		{
			return FRAME_CONSUMED
		},
		Frame::Text(TextInformationFrame {
			header: FrameHeader { id, .. },
			value: content,
			..
		}) if id.as_str() == "TPOS"
			&& split_pair(content, tag, ItemKey::DiscNumber, ItemKey::DiscTotal).is_some() =>
		{
			return FRAME_CONSUMED
		},
		Frame::Text(TextInformationFrame {
			header: FrameHeader { id, .. },
			value: content,
			..
		}) if id.as_str() == "MVIN"
			&& split_pair(
				content,
				tag,
				ItemKey::MovementNumber,
				ItemKey::MovementTotal,
			)
			.is_some() =>
		{
			return FRAME_CONSUMED
		},

		// TCON needs special treatment to translate genre IDs
		Frame::Text(TextInformationFrame {
			header: FrameHeader { id, .. },
			value: content,
			..
		}) if id.as_str() == "TCON" => {
			let genres = GenresIter::new(content, false);
			for genre in genres {
				tag.items.push(TagItem::new(
					ItemKey::Genre,
					ItemValue::Text(genre.to_string()),
				));
			}

			return FRAME_CONSUMED;
		},

		// TIPL needs special treatment, as we may not be able to consume all of its items
		Frame::KeyValue(KeyValueFrame {
			header: FrameHeader { id, .. },
			key_value_pairs,
			..
		}) if id.as_str() == "TIPL" => {
			key_value_pairs.retain_mut(|(key, value)| {
				for (item_key, tipl_key) in TIPL_MAPPINGS {
					if key == *tipl_key {
						tag.items.push(TagItem::new(
							item_key.clone(),
							ItemValue::Text(core::mem::take(value)),
						));
						return false; // This key-value pair is consumed
					}
				}

				true // Keep key-value pair
			});

			!key_value_pairs.is_empty() // Frame is consumed if we consumed all items
		},

		// TODO: HACK!! We are specifically disallowing descriptions with a length of 4.
		//       This is due to use storing 4 character IDs as Frame::Text on tag merge.
		//       Maybe ItemKey could use a "TXXX:" prefix eventually, so we would store
		//       "TXXX:MusicBrainz Album Id" instead of "MusicBrainz Album Id".
		// Store TXXX/WXXX frames by their descriptions, rather than their IDs
		Frame::UserText(ExtendedTextFrame {
			ref description,
			ref content,
			..
		}) if !description.is_empty() && description.len() != 4 => {
			let item_key = ItemKey::from_key(TagType::Id3v2, description);
			for c in content.split(V4_MULTI_VALUE_SEPARATOR) {
				tag.items.push(TagItem::new(
					item_key.clone(),
					ItemValue::Text(c.to_string()),
				));
			}

			return FRAME_CONSUMED;
		},
		Frame::UserUrl(ExtendedUrlFrame {
			ref description,
			ref content,
			..
		}) if !description.is_empty() && description.len() != 4 => {
			let item_key = ItemKey::from_key(TagType::Id3v2, description);
			for c in content.split(V4_MULTI_VALUE_SEPARATOR) {
				tag.items.push(TagItem::new(
					item_key.clone(),
					ItemValue::Locator(c.to_string()),
				));
			}

			return FRAME_CONSUMED;
		},

		Frame::UniqueFileIdentifier(UniqueFileIdentifierFrame {
			ref owner,
			ref identifier,
			..
		}) => {
			if owner != MUSICBRAINZ_UFID_OWNER {
				// Unsupported owner
				return FRAME_RETAINED;
			}

			let mut identifier = Cursor::new(identifier);
			let Ok(recording_id) = decode_text(
				&mut identifier,
				TextDecodeOptions::new().encoding(TextEncoding::Latin1),
			) else {
				return FRAME_RETAINED;
			};
			tag.items.push(TagItem::new(
				ItemKey::MusicBrainzRecordingId,
				ItemValue::Text(recording_id.content),
			));

			return FRAME_CONSUMED;
		},

		// COMM/USLT are identical frames, outside of their ID
		Frame::Comment(CommentFrame {
			header: FrameHeader{ id, .. },
			content,
			description,
			language,
			..
		})
		| Frame::UnsynchronizedText(UnsynchronizedTextFrame {
										header: FrameHeader{ id, .. },
			content,
			description,
			language,
			..
		}) => {
			let item_key = ItemKey::from_key(TagType::Id3v2, id.as_str());

			for c in content.split(V4_MULTI_VALUE_SEPARATOR) {
				let mut item = TagItem::new(item_key.clone(), ItemValue::Text(c.to_string()));

				item.set_lang(*language);

				if *description != EMPTY_CONTENT_DESCRIPTOR {
					item.set_description(std::mem::take(description));
				}

				tag.items.push(item);
			}
			return FRAME_CONSUMED;
		},

		Frame::Picture(AttachedPictureFrame {
			ref mut picture, ..
		}) => {
			tag.push_picture(std::mem::replace(picture, TOMBSTONE_PICTURE));
			return FRAME_CONSUMED;
		},

		Frame::Timestamp(TimestampFrame { header: FrameHeader {id, ..} , timestamp, .. }) => {
			let item_key = ItemKey::from_key(TagType::Id3v2, id.as_str());
			if matches!(item_key, ItemKey::Unknown(_)) {
				return FRAME_RETAINED;
			}

			if timestamp.verify().is_err() {
				return FRAME_RETAINED;
			}

			tag.items.push(TagItem::new(
				item_key,
				ItemValue::Text(timestamp.to_string()),
			));

			return FRAME_CONSUMED;
		},

		Frame::Text(TextInformationFrame { header: FrameHeader {id, .. }, value: content, .. }) => {
			let item_key = ItemKey::from_key(TagType::Id3v2, id.as_str());

			for c in content.split(V4_MULTI_VALUE_SEPARATOR) {
				tag.items.push(TagItem::new(
					item_key.clone(),
					ItemValue::Text(c.to_string()),
				));
			}
			return FRAME_CONSUMED;
		},
		Frame::Url(UrlLinkFrame {
			header: FrameHeader {id, .. },
			ref mut content, ..
		}) => {
			let item_key = ItemKey::from_key(TagType::Id3v2, id.as_str());
			tag.items.push(TagItem::new(
				item_key,
				ItemValue::Locator(std::mem::take(content)),
			));

			return FRAME_CONSUMED;
		},

		Frame::Binary(_)
		| Frame::UserText(_)
		| Frame::UserUrl(_) // Bare extended text/URL frames make no sense to support.
		| Frame::KeyValue(_)
		| Frame::RelativeVolumeAdjustment(_)
		| Frame::Ownership(_)
		| Frame::EventTimingCodes(_)
		| Frame::Popularimeter(_)
		| Frame::Private(_) => {
			return FRAME_RETAINED; // Keep unsupported frame
		},
	}
}

impl SplitTag for Id3v2Tag {
	type Remainder = SplitTagRemainder;

	fn split_tag(mut self) -> (Self::Remainder, Tag) {
		let mut tag = Tag::new(TagType::Id3v2);

		self.frames
			.retain_mut(|frame| handle_tag_split(&mut tag, frame));

		(SplitTagRemainder(self), tag)
	}
}

impl MergeTag for SplitTagRemainder {
	type Merged = Id3v2Tag;

	fn merge_tag(self, mut tag: Tag) -> Id3v2Tag {
		fn join_text_items<'a>(
			tag: &mut Tag,
			keys: impl IntoIterator<Item = &'a ItemKey>,
		) -> Option<String> {
			let mut concatenated: Option<String> = None;
			for key in keys {
				let mut iter = tag.take_strings(key);
				let Some(first) = iter.next() else {
					continue;
				};
				// Use the length of the first string for estimating the capacity
				// of the concatenated string.
				let estimated_len_per_item = first.len();
				let min_remaining_items = iter.size_hint().0;
				let concatenated = if let Some(concatenated) = &mut concatenated {
					concatenated.reserve(
						(1 + estimated_len_per_item) * (1 + min_remaining_items) + first.len(),
					);
					concatenated.push(V4_MULTI_VALUE_SEPARATOR);
					concatenated.push_str(&first);
					concatenated
				} else {
					let mut first = first;
					first.reserve((1 + estimated_len_per_item) * min_remaining_items);
					concatenated = Some(first);
					concatenated.as_mut().expect("some")
				};
				iter.for_each(|i| {
					concatenated.push(V4_MULTI_VALUE_SEPARATOR);
					concatenated.push_str(&i);
				});
			}
			concatenated
		}

		struct JoinedLanguageItem {
			lang: Lang,
			description: String,
			content: String,
		}

		fn join_language_items(
			tag: &mut Tag,
			key: &ItemKey,
		) -> Option<impl Iterator<Item = JoinedLanguageItem>> {
			let mut items = tag.take(key).collect::<Vec<_>>();
			if items.is_empty() {
				return None;
			}

			items.sort_by(|a, b| a.lang.cmp(&b.lang).then(a.description.cmp(&b.description)));

			let TagItem {
				lang: mut current_lang,
				description: mut current_description,
				item_value: ItemValue::Text(mut current_content),
				..
			} = items.remove(0)
			else {
				log::warn!("Expected a text item for {key:?}");
				return None;
			};

			let mut joined_items = Vec::<JoinedLanguageItem>::new();
			for item in items {
				let TagItem {
					lang,
					description,
					item_value: ItemValue::Text(content),
					..
				} = item
				else {
					continue;
				};

				if lang != current_lang || description != current_description {
					joined_items.push(JoinedLanguageItem {
						lang: current_lang,
						description: std::mem::take(&mut current_description),
						content: std::mem::take(&mut current_content),
					});
					current_lang = lang;
					current_description = description;
					current_content = content;
				} else {
					current_content.push(V4_MULTI_VALUE_SEPARATOR);
					current_content.push_str(&content);
				}
			}

			joined_items.push(JoinedLanguageItem {
				lang: current_lang,
				description: current_description,
				content: current_content,
			});

			Some(joined_items.into_iter())
		}

		let Self(mut merged) = self;
		merged.frames.reserve(tag.item_count() as usize);

		// Multi-valued text key-to-frame mappings
		// TODO: Extend this list of item keys as needed or desired
		for item_key in [
			&ItemKey::TrackArtist,
			&ItemKey::AlbumArtist,
			&ItemKey::TrackTitle,
			&ItemKey::AlbumTitle,
			&ItemKey::SetSubtitle,
			&ItemKey::TrackSubtitle,
			&ItemKey::OriginalAlbumTitle,
			&ItemKey::OriginalArtist,
			&ItemKey::OriginalLyricist,
			&ItemKey::ContentGroup,
			&ItemKey::AppleId3v2ContentGroup,
			&ItemKey::Genre,
			&ItemKey::Mood,
			&ItemKey::Composer,
			&ItemKey::Conductor,
			&ItemKey::Writer,
			&ItemKey::Director,
			&ItemKey::Lyricist,
			&ItemKey::MusicianCredits,
			&ItemKey::InternetRadioStationName,
			&ItemKey::InternetRadioStationOwner,
			&ItemKey::Remixer,
			&ItemKey::Work,
			&ItemKey::Movement,
			&ItemKey::FileOwner,
			&ItemKey::CopyrightMessage,
			&ItemKey::Language,
		] {
			let frame_id = item_key
				.map_key(TagType::Id3v2, false)
				.expect("valid frame id");
			if let Some(text) = join_text_items(&mut tag, [item_key]) {
				let frame = new_text_frame(FrameId::Valid(Cow::Borrowed(frame_id)), text);
				// Optimization: No duplicate checking according to the preconditions
				debug_assert!(!merged.frames.contains(&frame));
				merged.frames.push(frame);
			}
		}

		// Multi-valued TXXX key-to-frame mappings
		#[allow(clippy::single_element_loop)]
		for item_key in [&ItemKey::TrackArtists] {
			let frame_id = item_key
				.map_key(TagType::Id3v2, false)
				.expect("valid frame id");
			if let Some(text) = join_text_items(&mut tag, [item_key]) {
				let frame = new_user_text_frame(String::from(frame_id), text);
				// Optimization: No duplicate checking according to the preconditions
				debug_assert!(!merged.frames.contains(&frame));
				merged.frames.push(frame);
			}
		}

		// Multi-valued Label/Publisher key-to-frame mapping
		{
			let frame_id = ItemKey::Label
				.map_key(TagType::Id3v2, false)
				.expect("valid frame id");
			debug_assert_eq!(
				Some(frame_id),
				ItemKey::Publisher.map_key(TagType::Id3v2, false)
			);
			if let Some(text) = join_text_items(&mut tag, &[ItemKey::Label, ItemKey::Publisher]) {
				let frame = new_text_frame(FrameId::Valid(Cow::Borrowed(frame_id)), text);
				// Optimization: No duplicate checking according to the preconditions
				debug_assert!(!merged.frames.contains(&frame));
				merged.frames.push(frame);
			}
		}

		// Comment/Unsync text
		for key in [ItemKey::Comment, ItemKey::Lyrics] {
			let Some(items) = join_language_items(&mut tag, &key) else {
				continue;
			};

			for JoinedLanguageItem {
				lang,
				description,
				content,
			} in items
			{
				let frame = match key {
					ItemKey::Comment => Frame::Comment(CommentFrame::new(
						TextEncoding::UTF8,
						lang,
						description,
						content,
					)),
					ItemKey::Lyrics => Frame::UnsynchronizedText(UnsynchronizedTextFrame::new(
						TextEncoding::UTF8,
						lang,
						description,
						content,
					)),
					_ => continue,
				};
				// Optimization: No duplicate checking according to the preconditions
				debug_assert!(!merged.frames.contains(&frame));
				merged.frames.push(frame);
			}
		}

		// TIPL key-value mappings
		'tipl: {
			let mut key_value_pairs = Vec::new();
			for (item_key, tipl_key) in TIPL_MAPPINGS {
				for value in tag.take_strings(item_key) {
					key_value_pairs.push(((*tipl_key).to_string(), value));
				}
			}

			if key_value_pairs.is_empty() {
				break 'tipl;
			}

			// Check for an existing TIPL frame, and simply extend the existing list
			// to retain the current `TextEncoding` and `FrameFlags`.
			let existing_tipl = merged.take_first(&FrameId::Valid(Cow::Borrowed("TIPL")));
			if let Some(mut tipl_frame) = existing_tipl {
				if let Frame::KeyValue(KeyValueFrame {
					key_value_pairs: ref mut existing,
					..
				}) = &mut tipl_frame
				{
					existing.extend(key_value_pairs);
				}

				merged.frames.push(tipl_frame);
				break 'tipl;
			}

			merged.frames.push(new_key_value_frame(
				FrameId::Valid(Cow::Borrowed(INVOLVED_PEOPLE_LIST_ID)),
				key_value_pairs,
			));
		}

		// Flag items
		for item_key in [&ItemKey::FlagCompilation, &ItemKey::FlagPodcast] {
			let Some(text) = tag.take_strings(item_key).next() else {
				continue;
			};

			let Some(flag_value) = flag_item(&text) else {
				continue;
			};

			let frame_id = item_key
				.map_key(TagType::Id3v2, false)
				.expect("valid frame id");

			merged.frames.push(new_text_frame(
				FrameId::Valid(Cow::Borrowed(frame_id)),
				u8::from(flag_value).to_string(),
			));
		}

		'rate: {
			if let Some(advisory_rating) = tag.take_strings(&ItemKey::ParentalAdvisory).next() {
				let Ok(rating) = advisory_rating.parse::<u8>() else {
					log::warn!(
						"Parental advisory rating is not a number: {advisory_rating}, discarding"
					);
					break 'rate;
				};

				let Ok(parsed_rating) = AdvisoryRating::try_from(rating) else {
					log::warn!("Parental advisory rating is out of range: {rating}, discarding");
					break 'rate;
				};

				merged.frames.push(new_user_text_frame(
					"ITUNESADVISORY".to_string(),
					parsed_rating.as_u8().to_string(),
				));
			}
		}

		// Timestamps
		for item_key in [&ItemKey::RecordingDate, &ItemKey::OriginalReleaseDate] {
			let Some(text) = tag.take_strings(item_key).next() else {
				continue;
			};

			let frame_id = item_key
				.map_key(TagType::Id3v2, false)
				.expect("valid frame id");

			let frame;
			match Timestamp::from_str(&text) {
				Ok(timestamp) => {
					frame = new_timestamp_frame(FrameId::Valid(Cow::Borrowed(frame_id)), timestamp);
				},
				Err(_) => {
					// We can just preserve it as a text frame
					frame = new_text_frame(FrameId::Valid(Cow::Borrowed(frame_id)), text);
				},
			}

			merged.insert(frame);
		}

		// Insert all remaining items as single frames and deduplicate as needed
		for item in tag.items {
			merged.insert_item(item);
		}

		// Insert all pictures as single frames and deduplicate as needed
		for picture in tag.pictures {
			let frame = new_picture_frame(picture);
			if let Some(replaced) = merged.insert(frame) {
				log::warn!("Replaced picture frame: {replaced:?}");
			}
		}

		merged
	}
}

impl From<Id3v2Tag> for Tag {
	fn from(input: Id3v2Tag) -> Self {
		let (remainder, mut tag) = input.split_tag();

		if unsafe { global_options().preserve_format_specific_items } && remainder.0.len() > 0 {
			tag.companion_tag = Some(CompanionTag::Id3v2(remainder.0));
		}

		tag
	}
}

impl From<Tag> for Id3v2Tag {
	fn from(mut input: Tag) -> Self {
		if unsafe { global_options().preserve_format_specific_items } {
			if let Some(companion) = input.companion_tag.take().and_then(CompanionTag::id3v2) {
				return SplitTagRemainder(companion).merge_tag(input);
			}
		}

		SplitTagRemainder::default().merge_tag(input)
	}
}

pub(crate) struct Id3v2TagRef<'a, I: Iterator<Item = FrameRef<'a>> + 'a> {
	pub(crate) flags: Id3v2TagFlags,
	pub(crate) frames: Peekable<I>,
}

impl<'a> Id3v2TagRef<'a, std::iter::Empty<FrameRef<'a>>> {
	pub(crate) fn empty() -> Self {
		Self {
			flags: Id3v2TagFlags::default(),
			frames: std::iter::empty().peekable(),
		}
	}
}

// Create an iterator of FrameRef from a Tag's items for Id3v2TagRef::new
pub(crate) fn tag_frames(tag: &Tag) -> impl Iterator<Item = FrameRef<'_>> {
	#[derive(Clone)]
	enum CompanionTagIter<F, E> {
		Filled(F),
		Empty(E),
	}

	impl<'a, I> Iterator for CompanionTagIter<I, std::iter::Empty<Frame<'_>>>
	where
		I: Iterator<Item = FrameRef<'a>>,
	{
		type Item = FrameRef<'a>;

		fn next(&mut self) -> Option<Self::Item> {
			match self {
				CompanionTagIter::Filled(iter) => iter.next(),
				CompanionTagIter::Empty(_) => None,
			}
		}
	}

	fn create_frameref_for_number_pair<'a>(
		number: Option<&str>,
		total: Option<&str>,
		id: &'a str,
	) -> Option<FrameRef<'a>> {
		format_number_pair(number, total)
			.map(|value| FrameRef(Cow::Owned(Frame::text(Cow::Borrowed(id), value))))
	}

	fn create_framerefs_for_companion_tag(
		companion: Option<&CompanionTag>,
	) -> impl IntoIterator<Item = FrameRef<'_>> + Clone {
		match companion {
			Some(CompanionTag::Id3v2(companion)) => {
				CompanionTagIter::Filled(companion.frames.iter().filter_map(Frame::as_opt_ref))
			},
			_ => CompanionTagIter::Empty(std::iter::empty()),
		}
	}

	let items = tag
		.items()
		.filter(|item| !NUMBER_PAIR_KEYS.contains(item.key()))
		.map(TryInto::<FrameRef<'_>>::try_into)
		.filter_map(Result::ok)
		.chain(create_frameref_for_number_pair(
			tag.get_string(&ItemKey::TrackNumber),
			tag.get_string(&ItemKey::TrackTotal),
			"TRCK",
		))
		.chain(create_frameref_for_number_pair(
			tag.get_string(&ItemKey::DiscNumber),
			tag.get_string(&ItemKey::DiscTotal),
			"TPOS",
		))
		.chain(create_framerefs_for_companion_tag(
			tag.companion_tag.as_ref(),
		));

	let pictures = tag.pictures().iter().map(|p| {
		FrameRef(Cow::Owned(Frame::Picture(AttachedPictureFrame::new(
			TextEncoding::UTF8,
			p.clone(),
		))))
	});

	items.chain(pictures)
}

impl<'a, I: Iterator<Item = FrameRef<'a>> + 'a> Id3v2TagRef<'a, I> {
	pub(crate) fn write_to<F>(&mut self, file: &mut F, write_options: WriteOptions) -> Result<()>
	where
		F: FileLike,
		LoftyError: From<<F as Truncate>::Error>,
		LoftyError: From<<F as Length>::Error>,
	{
		super::write::write_id3v2(file, self, write_options)
	}

	pub(crate) fn dump_to<W: Write>(
		&mut self,
		writer: &mut W,
		write_options: WriteOptions,
	) -> Result<()> {
		let temp = super::write::create_tag(self, write_options)?;
		writer.write_all(&temp)?;

		Ok(())
	}
}
