#[cfg(test)]
mod tests;

use super::frame::id::FrameId;
use super::frame::{Frame, FrameFlags, FrameValue, EMPTY_CONTENT_DESCRIPTOR, UNKNOWN_LANGUAGE};
use super::header::{Id3v2TagFlags, Id3v2Version};
use crate::config::WriteOptions;
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
use crate::id3::v2::KeyValueFrame;
use crate::picture::{Picture, PictureType, TOMBSTONE_PICTURE};
use crate::tag::{try_parse_year, ItemKey, ItemValue, Tag, TagExt, TagItem, TagType};
use crate::traits::{Accessor, MergeTag, SplitTag};
use crate::util::text::{decode_text, TextDecodeOptions, TextEncoding};

use std::borrow::Cow;
use std::fs::File;
use std::io::{Cursor, Write};
use std::ops::Deref;
use std::path::Path;

use lofty_attr::tag;

const USER_DEFINED_TEXT_FRAME_ID: &str = "TXXX";
const COMMENT_FRAME_ID: &str = "COMM";

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
						FrameFlags::default(),
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
/// ID3v2 has `GEOB` and `SYLT` frames, which are not parsed by default, instead storing them as [`FrameValue::Binary`].
/// They can easily be parsed with [`GeneralEncapsulatedObject::parse`](crate::id3::v2::GeneralEncapsulatedObject::parse)
/// and [`SynchronizedText::parse`](crate::id3::v2::SynchronizedText::parse) respectively, and converted back to binary with
/// [`GeneralEncapsulatedObject::as_bytes`](crate::id3::v2::GeneralEncapsulatedObject::as_bytes) and
/// [`SynchronizedText::as_bytes`](crate::id3::v2::SynchronizedText::as_bytes) for writing.
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
		self.frames.iter().find(|f| &f.id == id)
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
	/// use lofty::Accessor;
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
		if let Some(Frame {
			value: FrameValue::Text(TextInformationFrame { value, .. }),
			..
		}) = frame
		{
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
	/// use lofty::Accessor;
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
		if let Some(Frame {
			value: FrameValue::Text(TextInformationFrame { value, .. }),
			..
		}) = self.get(id)
		{
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
			.filter(|frame| frame.id.as_str() == "TXXX")
			.find_map(|frame| match frame {
				Frame {
					value:
						FrameValue::UserText(ExtendedTextFrame {
							description: desc,
							content,
							..
						}),
					..
				} if desc == description => Some(content.as_str()),
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
		self.insert(Frame {
			id: FrameId::Valid(Cow::Borrowed(USER_DEFINED_TEXT_FRAME_ID)),
			value: FrameValue::UserText(ExtendedTextFrame {
				encoding: TextEncoding::UTF8,
				description,
				content,
			}),
			flags: FrameFlags::default(),
		})
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
			let ret = self.remove(&frame.id).next();
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
				matches!(frame, Frame {
                     value:
                         FrameValue::UserText(ExtendedTextFrame {
                             description: desc, ..
                         }),
                     ..
                 } if desc == description)
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
	/// # fn main() -> lofty::Result<()> {
	/// let mut tag = Id3v2Tag::new();
	/// assert!(tag.is_empty());
	///
	/// // Add a new "TMOO" frame
	/// let tmoo_frame = Frame::new(
	/// 	MOOD_FRAME_ID,
	/// 	TextInformationFrame {
	/// 		encoding: TextEncoding::Latin1,
	/// 		value: String::from("Classical"),
	/// 	},
	/// 	FrameFlags::default(),
	/// )?;
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
			if &self.frames[read_idx].id == id {
				self.frames.swap(split_idx, read_idx);
				split_idx += 1;
			}
		}

		self.frames.drain(..split_idx)
	}

	fn take_first(&mut self, id: &FrameId<'_>) -> Option<Frame<'static>> {
		self.frames
			.iter()
			.position(|f| &f.id == id)
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
					Frame {
						id: FrameId::Valid(id),
						value:
							FrameValue::Picture(AttachedPictureFrame {
								picture: Picture { pic_type, .. },
								..
							}),
						..
					} if id == "APIC" && pic_type == &picture.pic_type => {
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

		self.frames
			.push(new_picture_frame(picture, FrameFlags::default()));

		ret
	}

	/// Removes a certain [`PictureType`]
	pub fn remove_picture_type(&mut self, picture_type: PictureType) {
		self.frames.retain(|f| {
			!matches!(f, Frame {
					id: FrameId::Valid(id),
					value: FrameValue::Picture(AttachedPictureFrame {
						picture: Picture {
							pic_type: p_ty,
							..
						}, ..
					}),
					..
				} if id == "APIC" && p_ty == &picture_type)
		})
	}

	/// Returns all `USLT` frames
	pub fn unsync_text(&self) -> impl Iterator<Item = &UnsynchronizedTextFrame> + Clone {
		self.frames.iter().filter_map(|f| match f {
			Frame {
				id: FrameId::Valid(id),
				value: FrameValue::UnsynchronizedText(val),
				..
			} if id == "USLT" => Some(val),
			_ => None,
		})
	}

	/// Returns all `COMM` frames with an empty content descriptor
	pub fn comments(&self) -> impl Iterator<Item = &CommentFrame> {
		self.frames.iter().filter_map(|frame| {
			filter_comment_frame_by_description(frame, &EMPTY_CONTENT_DESCRIPTOR)
		})
	}

	fn split_num_pair(&self, id: &FrameId<'_>) -> (Option<u32>, Option<u32>) {
		if let Some(Frame {
			value: FrameValue::Text(TextInformationFrame { ref value, .. }),
			..
		}) = self.get(id)
		{
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
		if let Some(Frame {
			value: FrameValue::Text(TextInformationFrame { ref value, .. }),
			..
		}) = self.get(&GENRE_ID)
		{
			return Some(GenresIter::new(value));
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

struct GenresIter<'a> {
	value: &'a str,
	pos: usize,
}

impl<'a> GenresIter<'a> {
	pub fn new(value: &'a str) -> GenresIter<'_> {
		GenresIter { value, pos: 0 }
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
			return Some(parse_genre(&self.value[start..end]));
		}

		if remainder.starts_with('(') && remainder.contains(')') {
			let start = self.pos + 1;
			let mut end = self.pos + remainder.find(')').unwrap();
			self.pos = end + 1;
			// handle bracketed refinement e.g. (55)((I think...)"
			if remainder.starts_with("((") {
				end += 1;
			}
			return Some(parse_genre(&self.value[start..end]));
		}

		self.pos = self.value.len();
		Some(parse_genre(remainder))
	}
}

fn parse_genre(genre: &str) -> &str {
	if genre.len() > 3 {
		return genre;
	}
	if let Ok(id) = genre.parse::<usize>() {
		if id < GENRES.len() {
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
) -> Option<&'a CommentFrame> {
	match &frame.value {
		FrameValue::Comment(comment_frame) if frame.id_str() == COMMENT_FRAME_ID => {
			(comment_frame.description == description).then_some(comment_frame)
		},
		_ => None,
	}
}

fn filter_comment_frame_by_description_mut<'a>(
	frame: &'a mut Frame<'_>,
	description: &str,
) -> Option<&'a mut CommentFrame> {
	if frame.id_str() != COMMENT_FRAME_ID {
		return None;
	}
	match &mut frame.value {
		FrameValue::Comment(comment_frame) => {
			(comment_frame.description == description).then_some(comment_frame)
		},
		_ => None,
	}
}

fn new_text_frame(id: FrameId<'_>, value: String, flags: FrameFlags) -> Frame<'_> {
	Frame {
		id,
		value: FrameValue::Text(TextInformationFrame {
			encoding: TextEncoding::UTF8,
			value,
		}),
		flags,
	}
}

fn new_comment_frame(content: String, flags: FrameFlags) -> Frame<'static> {
	Frame {
		id: FrameId::Valid(Cow::Borrowed(COMMENT_FRAME_ID)),
		value: FrameValue::Comment(CommentFrame {
			encoding: TextEncoding::UTF8,
			language: UNKNOWN_LANGUAGE,
			description: EMPTY_CONTENT_DESCRIPTOR,
			content,
		}),
		flags,
	}
}

fn new_picture_frame(picture: Picture, flags: FrameFlags) -> Frame<'static> {
	Frame {
		id: FrameId::Valid(Cow::Borrowed("APIC")),
		value: FrameValue::Picture(AttachedPictureFrame {
			encoding: TextEncoding::UTF8,
			picture,
		}),
		flags,
	}
}

const TITLE_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("TIT2"));
const ARTIST_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("TPE1"));
const ALBUM_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("TALB"));
const GENRE_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("TCON"));
const TRACK_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("TRCK"));
const DISC_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("TPOS"));
const RECORDING_TIME_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("TDRC"));

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
		self.insert(new_text_frame(GENRE_ID, value, FrameFlags::default()));
	}

	fn remove_genre(&mut self) {
		let _ = self.remove(&GENRE_ID);
	}

	fn year(&self) -> Option<u32> {
		if let Some(Frame {
			value: FrameValue::Text(TextInformationFrame { value, .. }),
			..
		}) = self.get(&RECORDING_TIME_ID)
		{
			return try_parse_year(value);
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
			self.frames
				.push(new_comment_frame(value, FrameFlags::default()));
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

	fn len(&self) -> usize {
		self.frames.len()
	}

	fn contains<'a>(&'a self, key: Self::RefKey<'a>) -> bool {
		self.frames.iter().any(|frame| &frame.id == key)
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
	/// * Attempting to write an invalid [`FrameId`]/[`FrameValue`] pairing
	fn save_to(
		&self,
		file: &mut File,
		write_options: WriteOptions,
	) -> std::result::Result<(), Self::Err> {
		Id3v2TagRef {
			flags: self.flags,
			frames: self.frames.iter().filter_map(Frame::as_opt_ref),
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
			frames: self.frames.iter().filter_map(Frame::as_opt_ref),
		}
		.dump_to(writer, write_options)
	}

	fn remove_from_path<P: AsRef<Path>>(&self, path: P) -> std::result::Result<(), Self::Err> {
		TagType::Id3v2.remove_from_path(path)
	}

	fn remove_from(&self, file: &mut File) -> std::result::Result<(), Self::Err> {
		TagType::Id3v2.remove_from(file)
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

impl SplitTag for Id3v2Tag {
	type Remainder = SplitTagRemainder;

	fn split_tag(mut self) -> (Self::Remainder, Tag) {
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

			let mut split =
				content.splitn(2, &[V4_MULTI_VALUE_SEPARATOR, NUMBER_PAIR_SEPARATOR][..]);

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

		let mut tag = Tag::new(TagType::Id3v2);

		self.frames.retain_mut(|frame| {
			let id = &frame.id;

			// The text pairs need some special treatment
			match (id.as_str(), &mut frame.value) {
				("TRCK", FrameValue::Text(TextInformationFrame { value: content, .. }))
					if split_pair(content, &mut tag, ItemKey::TrackNumber, ItemKey::TrackTotal)
						.is_some() =>
				{
					false // Frame consumed
				},
				("TPOS", FrameValue::Text(TextInformationFrame { value: content, .. }))
					if split_pair(content, &mut tag, ItemKey::DiscNumber, ItemKey::DiscTotal)
						.is_some() =>
				{
					false // Frame consumed
				},
				("MVIN", FrameValue::Text(TextInformationFrame { value: content, .. }))
					if split_pair(
						content,
						&mut tag,
						ItemKey::MovementNumber,
						ItemKey::MovementTotal,
					)
					.is_some() =>
				{
					false // Frame consumed
				},
				// TCON needs special treatment to translate genre IDs
				("TCON", FrameValue::Text(TextInformationFrame { value: content, .. })) => {
					let genres = GenresIter::new(content);
					for genre in genres {
						tag.items.push(TagItem::new(
							ItemKey::Genre,
							ItemValue::Text(genre.to_string()),
						));
					}
					false // Frame consumed
				},
				(
					"TIPL",
					FrameValue::KeyValue(KeyValueFrame {
						key_value_pairs, ..
					}),
				) => {
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
				// Store TXXX/WXXX frames by their descriptions, rather than their IDs
				(
					"TXXX",
					FrameValue::UserText(ExtendedTextFrame {
						ref description,
						ref content,
						..
					}),
				) => {
					let item_key = ItemKey::from_key(TagType::Id3v2, description);
					for c in content.split(V4_MULTI_VALUE_SEPARATOR) {
						tag.items.push(TagItem::new(
							item_key.clone(),
							ItemValue::Text(c.to_string()),
						));
					}
					false // Frame consumed
				},
				(
					"WXXX",
					FrameValue::UserUrl(ExtendedUrlFrame {
						ref description,
						ref content,
						..
					}),
				) => {
					let item_key = ItemKey::from_key(TagType::Id3v2, description);
					for c in content.split(V4_MULTI_VALUE_SEPARATOR) {
						tag.items.push(TagItem::new(
							item_key.clone(),
							ItemValue::Locator(c.to_string()),
						));
					}
					false // Frame consumed
				},
				(
					"UFID",
					FrameValue::UniqueFileIdentifier(UniqueFileIdentifierFrame {
						ref owner,
						ref identifier,
						..
					}),
				) => {
					if owner == MUSICBRAINZ_UFID_OWNER {
						let mut identifier = Cursor::new(identifier);
						let Ok(recording_id) = decode_text(
							&mut identifier,
							TextDecodeOptions::new().encoding(TextEncoding::Latin1),
						) else {
							return true; // Keep frame
						};
						tag.items.push(TagItem::new(
							ItemKey::MusicBrainzRecordingId,
							ItemValue::Text(recording_id.content),
						));
						false // Frame consumed
					} else {
						// Unsupported owner
						true // Keep frame
					}
				},
				(id, value) => {
					let item_key = ItemKey::from_key(TagType::Id3v2, id);

					let item_value = match value {
						FrameValue::Comment(CommentFrame {
							content,
							description,
							..
						})
						| FrameValue::UnsynchronizedText(UnsynchronizedTextFrame {
							content,
							description,
							..
						})
						| FrameValue::UserText(ExtendedTextFrame {
							content,
							description,
							..
						}) => {
							if *description == EMPTY_CONTENT_DESCRIPTOR {
								for c in content.split(V4_MULTI_VALUE_SEPARATOR) {
									tag.items.push(TagItem::new(
										item_key.clone(),
										ItemValue::Text(c.to_string()),
									));
								}
								return false; // Frame consumed
							}
							// ...else do not convert text frames with a non-empty content
							// descriptor that would otherwise unintentionally be modified
							// and corrupted by the incomplete conversion into a [`TagItem`].
							// TODO: How to convert these frames consistently and safely
							// such that the content descriptor is preserved during read/write
							// round trips?
							return true; // Keep frame
						},
						FrameValue::Text(TextInformationFrame { value: content, .. }) => {
							for c in content.split(V4_MULTI_VALUE_SEPARATOR) {
								tag.items.push(TagItem::new(
									item_key.clone(),
									ItemValue::Text(c.to_string()),
								));
							}
							return false; // Frame consumed
						},
						FrameValue::Url(UrlLinkFrame(content))
						| FrameValue::UserUrl(ExtendedUrlFrame { content, .. }) => {
							ItemValue::Locator(std::mem::take(content))
						},
						FrameValue::Picture(AttachedPictureFrame { picture, .. }) => {
							tag.push_picture(std::mem::replace(picture, TOMBSTONE_PICTURE));
							return false; // Frame consumed
						},
						FrameValue::Popularimeter(popularimeter) => {
							ItemValue::Binary(popularimeter.as_bytes())
						},
						FrameValue::Binary(binary) => ItemValue::Binary(std::mem::take(binary)),
						FrameValue::KeyValue(_)
						| FrameValue::UniqueFileIdentifier(_)
						| FrameValue::RelativeVolumeAdjustment(_)
						| FrameValue::Ownership(_)
						| FrameValue::EventTimingCodes(_)
						| FrameValue::Private(_) => {
							return true; // Keep unsupported frame
						},
					};

					tag.items.push(TagItem::new(item_key, item_value));
					false // Frame consumed
				},
			}
		});

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
			&ItemKey::Lyrics,
		] {
			let frame_id = item_key
				.map_key(TagType::Id3v2, false)
				.expect("valid frame id");
			if let Some(text) = join_text_items(&mut tag, [item_key]) {
				let frame = new_text_frame(
					FrameId::Valid(Cow::Borrowed(frame_id)),
					text,
					FrameFlags::default(),
				);
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
				let frame = new_text_frame(
					FrameId::Valid(Cow::Borrowed(frame_id)),
					text,
					FrameFlags::default(),
				);
				// Optimization: No duplicate checking according to the preconditions
				debug_assert!(!merged.frames.contains(&frame));
				merged.frames.push(frame);
			}
		}

		// Multi-valued Comment key-to-frame mapping
		if let Some(text) = join_text_items(&mut tag, &[ItemKey::Comment]) {
			let frame = new_comment_frame(text, FrameFlags::default());
			// Optimization: No duplicate checking according to the preconditions
			debug_assert!(!merged.frames.contains(&frame));
			merged.frames.push(frame);
		};

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
				if let FrameValue::KeyValue(KeyValueFrame {
					key_value_pairs: ref mut existing,
					..
				}) = &mut tipl_frame.value
				{
					existing.extend(key_value_pairs);
				}

				merged.frames.push(tipl_frame);
				break 'tipl;
			}

			merged.frames.push(Frame {
				id: FrameId::Valid(Cow::Borrowed("TIPL")),
				value: FrameValue::KeyValue(KeyValueFrame {
					key_value_pairs,
					encoding: TextEncoding::UTF8,
				}),
				flags: FrameFlags::default(),
			});
		}

		// Flag items
		for item_key in [&ItemKey::FlagCompilation, &ItemKey::FlagPodcast] {
			let Some(flag_value) = tag.take_strings(item_key).next() else {
				continue;
			};

			if flag_value != "1" && flag_value != "0" {
				continue;
			}

			let frame_id = item_key
				.map_key(TagType::Id3v2, false)
				.expect("valid frame id");

			merged.frames.push(new_text_frame(
				FrameId::Valid(Cow::Borrowed(frame_id)),
				flag_value,
				FrameFlags::default(),
			));
		}

		// Insert all remaining items as single frames and deduplicate as needed
		for item in tag.items {
			merged.insert_item(item);
		}

		// Insert all pictures as single frames and deduplicate as needed
		for picture in tag.pictures {
			let frame = new_picture_frame(picture, FrameFlags::default());
			if let Some(replaced) = merged.insert(frame) {
				log::warn!("Replaced picture frame: {replaced:?}");
			}
		}

		merged
	}
}

impl From<Id3v2Tag> for Tag {
	fn from(input: Id3v2Tag) -> Self {
		input.split_tag().1
	}
}

impl From<Tag> for Id3v2Tag {
	fn from(input: Tag) -> Self {
		SplitTagRemainder::default().merge_tag(input)
	}
}

pub(crate) struct Id3v2TagRef<'a, I: Iterator<Item = FrameRef<'a>> + 'a> {
	pub(crate) flags: Id3v2TagFlags,
	pub(crate) frames: I,
}

impl<'a> Id3v2TagRef<'a, std::iter::Empty<FrameRef<'a>>> {
	pub(crate) fn empty() -> Self {
		Self {
			flags: Id3v2TagFlags::default(),
			frames: std::iter::empty(),
		}
	}
}

// Create an iterator of FrameRef from a Tag's items for Id3v2TagRef::new
pub(crate) fn tag_frames(tag: &Tag) -> impl Iterator<Item = FrameRef<'_>> + Clone {
	fn create_frameref_for_number_pair<'a>(
		number: Option<&str>,
		total: Option<&str>,
		id: &'a str,
	) -> Option<FrameRef<'a>> {
		format_number_pair(number, total).map(|value| {
			let frame = Frame::text(Cow::Borrowed(id), value);

			FrameRef {
				id: frame.id,
				value: Cow::Owned(frame.value),
				flags: frame.flags,
			}
		})
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
		));

	let pictures = tag.pictures().iter().map(|p| FrameRef {
		id: FrameId::Valid(Cow::Borrowed("APIC")),
		value: Cow::Owned(FrameValue::Picture(AttachedPictureFrame {
			encoding: TextEncoding::UTF8,
			picture: p.clone(),
		})),
		flags: FrameFlags::default(),
	});

	items.chain(pictures)
}

impl<'a, I: Iterator<Item = FrameRef<'a>> + Clone + 'a> Id3v2TagRef<'a, I> {
	pub(crate) fn write_to(&mut self, file: &mut File, write_options: WriteOptions) -> Result<()> {
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
