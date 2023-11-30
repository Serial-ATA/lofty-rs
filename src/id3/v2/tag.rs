use super::frame::id::FrameId;
use super::frame::{Frame, FrameFlags, FrameValue, EMPTY_CONTENT_DESCRIPTOR, UNKNOWN_LANGUAGE};
use super::header::{Id3v2TagFlags, Id3v2Version};
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
use crate::tag::item::{ItemKey, ItemValue, TagItem};
use crate::tag::{try_parse_year, Tag, TagType};
use crate::traits::{Accessor, MergeTag, SplitTag, TagExt};
use crate::util::text::{decode_text, TextEncoding};

use std::borrow::Cow;
use std::convert::TryInto;
use std::fs::File;
use std::io::{Cursor, Write};
use std::ops::Deref;
use std::path::Path;

use lofty_attr::tag;

const USER_DEFINED_TEXT_FRAME_ID: &str = "TXXX";
const COMMENT_FRAME_ID: &str = "COMM";

const V4_MULTI_VALUE_SEPARATOR: char = '\0';

macro_rules! impl_accessor {
	($($name:ident => $id:literal;)+) => {
		paste::paste! {
			$(
				fn $name(&self) -> Option<Cow<'_, str>> {
					self.get_text(&[<$name:upper _ID>])
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
	/// use lofty::TagExt;
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
	/// NOTE: This will not work for `TXXX` frames, use [`Id3v2Tag::get_user_text`] for that.
	///
	/// If the tag is [`Id3v2Version::V4`], this will allocate if the text contains any
	/// null (`'\0'`) text separators to replace them with a slash (`'/'`).
	pub fn get_text(&self, id: &FrameId<'_>) -> Option<Cow<'_, str>> {
		let frame = self.get(id);
		if let Some(Frame {
			value: FrameValue::Text(TextInformationFrame { value, .. }),
			..
		}) = frame
		{
			if !value.contains(V4_MULTI_VALUE_SEPARATOR)
				|| self.original_version != Id3v2Version::V4
			{
				return Some(Cow::Borrowed(value.as_str()));
			}

			return Some(Cow::Owned(value.replace(V4_MULTI_VALUE_SEPARATOR, "/")));
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
			return Some(value.split('\0'));
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
	/// use lofty::TagExt;
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
	/// use lofty::TagExt;
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
	/// use lofty::{TagExt, TextEncoding};
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
	fn save_to(&self, file: &mut File) -> std::result::Result<(), Self::Err> {
		Id3v2TagRef {
			flags: self.flags,
			frames: self.frames.iter().filter_map(Frame::as_opt_ref),
		}
		.write_to(file)
	}

	/// Dumps the tag to a writer
	///
	/// # Errors
	///
	/// * [`std::io::Error`]
	/// * [`ErrorKind::TooMuchData`](crate::error::ErrorKind::TooMuchData)
	fn dump_to<W: Write>(&self, writer: &mut W) -> std::result::Result<(), Self::Err> {
		Id3v2TagRef {
			flags: self.flags,
			frames: self.frames.iter().filter_map(Frame::as_opt_ref),
		}
		.dump_to(writer)
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
						for (item_key, tipl_key) in TIPL_MAPPINGS.iter() {
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
						let Ok(recording_id) =
							decode_text(&mut identifier, TextEncoding::Latin1, false)
						else {
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
	pub(crate) fn write_to(&mut self, file: &mut File) -> Result<()> {
		super::write::write_id3v2(file, self)
	}

	pub(crate) fn dump_to<W: Write>(&mut self, writer: &mut W) -> Result<()> {
		let temp = super::write::create_tag(self)?;
		writer.write_all(&temp)?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use crate::ParsingMode;
	use std::borrow::Cow;

	use crate::id3::v2::frame::MUSICBRAINZ_UFID_OWNER;
	use crate::id3::v2::header::{Id3v2Header, Id3v2Version};
	use crate::id3::v2::items::{ExtendedUrlFrame, Popularimeter, UniqueFileIdentifierFrame};
	use crate::id3::v2::tag::{filter_comment_frame_by_description, new_text_frame};
	use crate::id3::v2::util::mappings::TIPL_MAPPINGS;
	use crate::id3::v2::util::pairs::DEFAULT_NUMBER_IN_PAIR;
	use crate::id3::v2::{
		AttachedPictureFrame, CommentFrame, ExtendedTextFrame, Frame, FrameFlags, FrameId,
		FrameValue, Id3v2Tag, KeyValueFrame, TextInformationFrame, UrlLinkFrame,
	};
	use crate::tag::utils::test_utils::read_path;
	use crate::util::text::TextEncoding;
	use crate::{
		Accessor as _, ItemKey, ItemValue, MergeTag as _, MimeType, Picture, PictureType,
		SplitTag as _, Tag, TagExt as _, TagItem, TagType,
	};

	use super::{COMMENT_FRAME_ID, EMPTY_CONTENT_DESCRIPTOR, GENRE_ID};

	fn read_tag(path: &str) -> Id3v2Tag {
		let tag_bytes = crate::tag::utils::test_utils::read_path(path);

		let mut reader = std::io::Cursor::new(&tag_bytes[..]);

		let header = Id3v2Header::parse(&mut reader).unwrap();
		crate::id3::v2::read::parse_id3v2(&mut reader, header, ParsingMode::Strict).unwrap()
	}

	#[test]
	fn parse_id3v2() {
		let mut expected_tag = Id3v2Tag::default();

		let encoding = TextEncoding::Latin1;
		let flags = FrameFlags::default();

		expected_tag.insert(
			Frame::new(
				"TPE1",
				FrameValue::Text(TextInformationFrame {
					encoding,
					value: String::from("Bar artist"),
				}),
				flags,
			)
			.unwrap(),
		);

		expected_tag.insert(
			Frame::new(
				"TIT2",
				FrameValue::Text(TextInformationFrame {
					encoding,
					value: String::from("Foo title"),
				}),
				flags,
			)
			.unwrap(),
		);

		expected_tag.insert(
			Frame::new(
				"TALB",
				FrameValue::Text(TextInformationFrame {
					encoding,
					value: String::from("Baz album"),
				}),
				flags,
			)
			.unwrap(),
		);

		expected_tag.insert(
			Frame::new(
				COMMENT_FRAME_ID,
				FrameValue::Comment(CommentFrame {
					encoding,
					language: *b"eng",
					description: EMPTY_CONTENT_DESCRIPTOR,
					content: String::from("Qux comment"),
				}),
				flags,
			)
			.unwrap(),
		);

		expected_tag.insert(
			Frame::new(
				"TDRC",
				FrameValue::Text(TextInformationFrame {
					encoding,
					value: String::from("1984"),
				}),
				flags,
			)
			.unwrap(),
		);

		expected_tag.insert(
			Frame::new(
				"TRCK",
				FrameValue::Text(TextInformationFrame {
					encoding,
					value: String::from("1"),
				}),
				flags,
			)
			.unwrap(),
		);

		expected_tag.insert(
			Frame::new(
				"TCON",
				FrameValue::Text(TextInformationFrame {
					encoding,
					value: String::from("Classical"),
				}),
				flags,
			)
			.unwrap(),
		);

		let parsed_tag = read_tag("tests/tags/assets/id3v2/test.id3v24");

		assert_eq!(expected_tag, parsed_tag);
	}

	#[test]
	fn id3v2_re_read() {
		let parsed_tag = read_tag("tests/tags/assets/id3v2/test.id3v24");

		let mut writer = Vec::new();
		parsed_tag.dump_to(&mut writer).unwrap();

		let temp_reader = &mut &*writer;

		let temp_header = Id3v2Header::parse(temp_reader).unwrap();
		let temp_parsed_tag =
			crate::id3::v2::read::parse_id3v2(temp_reader, temp_header, ParsingMode::Strict)
				.unwrap();

		assert_eq!(parsed_tag, temp_parsed_tag);
	}

	#[test]
	fn id3v2_to_tag() {
		let id3v2 = read_tag("tests/tags/assets/id3v2/test.id3v24");

		let tag: Tag = id3v2.into();

		crate::tag::utils::test_utils::verify_tag(&tag, true, true);
	}

	#[test]
	fn id3v2_to_tag_popm() {
		let id3v2 = read_tag("tests/tags/assets/id3v2/test_popm.id3v24");

		let tag: Tag = id3v2.into();

		assert_eq!(
			tag.get_binary(&ItemKey::Popularimeter, false),
			Some(
				&[
					b'f', b'o', b'o', b'@', b'b', b'a', b'r', b'.', b'c', b'o', b'm', 0, 196, 0, 0,
					255, 255,
				][..]
			),
		);
	}

	#[test]
	fn tag_to_id3v2_popm() {
		let mut tag = Tag::new(TagType::Id3v2);
		tag.insert(TagItem::new(
			ItemKey::Popularimeter,
			ItemValue::Binary(vec![
				b'f', b'o', b'o', b'@', b'b', b'a', b'r', b'.', b'c', b'o', b'm', 0, 196, 0, 0,
				255, 255,
			]),
		));

		let expected = Popularimeter {
			email: String::from("foo@bar.com"),
			rating: 196,
			counter: 65535,
		};

		let converted_tag: Id3v2Tag = tag.into();

		assert_eq!(converted_tag.frames.len(), 1);
		let actual_frame = converted_tag.frames.first().unwrap();

		assert_eq!(actual_frame.id, FrameId::Valid(Cow::Borrowed("POPM")));
		// Note: as POPM frames are considered equal by email alone, each field must
		// be separately validated
		match actual_frame.content() {
			FrameValue::Popularimeter(pop) => {
				assert_eq!(pop.email, expected.email);
				assert_eq!(pop.rating, expected.rating);
				assert_eq!(pop.counter, expected.counter);
			},
			_ => unreachable!(),
		}
	}

	#[test]
	fn fail_write_bad_frame() {
		let mut tag = Id3v2Tag::default();
		tag.insert(Frame {
			id: FrameId::Valid(Cow::Borrowed("ABCD")),
			value: FrameValue::Url(UrlLinkFrame(String::from("FOO URL"))),
			flags: FrameFlags::default(),
		});

		let res = tag.dump_to(&mut Vec::<u8>::new());

		assert!(res.is_err());
		assert_eq!(
			res.unwrap_err().to_string(),
			String::from(
				"ID3v2: Attempted to write an invalid frame. ID: \"ABCD\", Value: \"Url\""
			)
		);
	}

	#[test]
	fn tag_to_id3v2() {
		fn verify_frame(tag: &Id3v2Tag, id: &str, value: &str) {
			let frame = tag.get(&FrameId::Valid(Cow::Borrowed(id)));

			assert!(frame.is_some());

			let frame = frame.unwrap();

			assert_eq!(
				frame.content(),
				&FrameValue::Text(TextInformationFrame {
					encoding: TextEncoding::UTF8,
					value: String::from(value)
				})
			);
		}

		let tag = crate::tag::utils::test_utils::create_tag(TagType::Id3v2);

		let id3v2_tag: Id3v2Tag = tag.into();

		verify_frame(&id3v2_tag, "TIT2", "Foo title");
		verify_frame(&id3v2_tag, "TPE1", "Bar artist");
		verify_frame(&id3v2_tag, "TALB", "Baz album");

		let frame = id3v2_tag
			.get(&FrameId::Valid(Cow::Borrowed(COMMENT_FRAME_ID)))
			.unwrap();
		assert_eq!(
			frame.content(),
			&FrameValue::Comment(CommentFrame {
				encoding: TextEncoding::Latin1,
				language: *b"eng",
				description: EMPTY_CONTENT_DESCRIPTOR,
				content: String::from("Qux comment")
			})
		);

		verify_frame(&id3v2_tag, "TRCK", "1");
		verify_frame(&id3v2_tag, "TCON", "Classical");
	}

	#[allow(clippy::field_reassign_with_default)]
	fn create_full_test_tag(version: Id3v2Version) -> Id3v2Tag {
		let mut tag = Id3v2Tag::default();
		tag.original_version = version;

		let encoding = TextEncoding::UTF16;
		let flags = FrameFlags::default();

		tag.insert(Frame {
			id: FrameId::Valid(Cow::Borrowed("TIT2")),
			value: FrameValue::Text(TextInformationFrame {
				encoding,
				value: String::from("TempleOS Hymn Risen (Remix)"),
			}),
			flags,
		});

		tag.insert(Frame {
			id: FrameId::Valid(Cow::Borrowed("TPE1")),
			value: FrameValue::Text(TextInformationFrame {
				encoding,
				value: String::from("Dave Eddy"),
			}),
			flags,
		});

		tag.insert(Frame {
			id: FrameId::Valid(Cow::Borrowed("TRCK")),
			value: FrameValue::Text(TextInformationFrame {
				encoding: TextEncoding::Latin1,
				value: String::from("1"),
			}),
			flags,
		});

		tag.insert(Frame {
			id: FrameId::Valid(Cow::Borrowed("TALB")),
			value: FrameValue::Text(TextInformationFrame {
				encoding,
				value: String::from("Summer"),
			}),
			flags,
		});

		tag.insert(Frame {
			id: FrameId::Valid(Cow::Borrowed("TDRC")),
			value: FrameValue::Text(TextInformationFrame {
				encoding,
				value: String::from("2017"),
			}),
			flags,
		});

		tag.insert(Frame {
			id: FrameId::Valid(Cow::Borrowed("TCON")),
			value: FrameValue::Text(TextInformationFrame {
				encoding,
				value: String::from("Electronic"),
			}),
			flags,
		});

		tag.insert(Frame {
			id: FrameId::Valid(Cow::Borrowed("TLEN")),
			value: FrameValue::Text(TextInformationFrame {
				encoding: TextEncoding::UTF16,
				value: String::from("213017"),
			}),
			flags,
		});

		tag.insert(Frame {
			id: FrameId::Valid(Cow::Borrowed("APIC")),
			value: FrameValue::Picture(AttachedPictureFrame {
				encoding: TextEncoding::Latin1,
				picture: Picture {
					pic_type: PictureType::CoverFront,
					mime_type: Some(MimeType::Png),
					description: None,
					data: read_path("tests/tags/assets/id3v2/test_full_cover.png").into(),
				},
			}),
			flags,
		});

		tag
	}

	#[test]
	fn id3v24_full() {
		let tag = create_full_test_tag(Id3v2Version::V4);
		let parsed_tag = read_tag("tests/tags/assets/id3v2/test_full.id3v24");

		assert_eq!(tag, parsed_tag);
	}

	#[test]
	fn id3v23_full() {
		let tag = create_full_test_tag(Id3v2Version::V3);
		let parsed_tag = read_tag("tests/tags/assets/id3v2/test_full.id3v23");

		assert_eq!(tag, parsed_tag);
	}

	#[test]
	fn id3v22_full() {
		let tag = create_full_test_tag(Id3v2Version::V2);
		let parsed_tag = read_tag("tests/tags/assets/id3v2/test_full.id3v22");

		assert_eq!(tag, parsed_tag);
	}

	#[test]
	fn id3v24_footer() {
		let mut tag = create_full_test_tag(Id3v2Version::V4);
		tag.flags.footer = true;

		let mut writer = Vec::new();
		tag.dump_to(&mut writer).unwrap();

		let mut reader = &mut &writer[..];

		let header = Id3v2Header::parse(&mut reader).unwrap();
		assert!(crate::id3::v2::read::parse_id3v2(reader, header, ParsingMode::Strict).is_ok());

		assert_eq!(writer[3..10], writer[writer.len() - 7..])
	}

	#[test]
	fn issue_36() {
		let picture_data = vec![0; 200];

		let picture = Picture::new_unchecked(
			PictureType::CoverFront,
			Some(MimeType::Jpeg),
			Some(String::from("cover")),
			picture_data,
		);

		let mut tag = Tag::new(TagType::Id3v2);
		tag.push_picture(picture.clone());

		let mut writer = Vec::new();
		tag.dump_to(&mut writer).unwrap();

		let mut reader = &mut &writer[..];

		let header = Id3v2Header::parse(&mut reader).unwrap();
		let tag = crate::id3::v2::read::parse_id3v2(reader, header, ParsingMode::Strict).unwrap();

		assert_eq!(tag.len(), 1);
		assert_eq!(
			tag.frames.first(),
			Some(&Frame {
				id: FrameId::Valid(Cow::Borrowed("APIC")),
				value: FrameValue::Picture(AttachedPictureFrame {
					encoding: TextEncoding::UTF8,
					picture
				}),
				flags: FrameFlags::default()
			})
		);
	}

	#[test]
	fn popm_frame() {
		let parsed_tag = read_tag("tests/tags/assets/id3v2/test_popm.id3v24");

		assert_eq!(parsed_tag.frames.len(), 1);
		let popm_frame = parsed_tag.frames.first().unwrap();

		assert_eq!(popm_frame.id, FrameId::Valid(Cow::Borrowed("POPM")));
		assert_eq!(
			popm_frame.value,
			FrameValue::Popularimeter(Popularimeter {
				email: String::from("foo@bar.com"),
				rating: 196,
				counter: 65535
			})
		)
	}

	#[test]
	fn multi_value_frame_to_tag() {
		use crate::traits::Accessor;
		let mut tag = Id3v2Tag::default();

		tag.set_artist(String::from("foo\0bar\0baz"));

		let tag: Tag = tag.into();
		let collected_artists = tag.get_strings(&ItemKey::TrackArtist).collect::<Vec<_>>();
		assert_eq!(&collected_artists, &["foo", "bar", "baz"])
	}

	#[test]
	fn multi_item_tag_to_id3v2() {
		use crate::traits::Accessor;
		let mut tag = Tag::new(TagType::Id3v2);

		tag.push_unchecked(TagItem::new(
			ItemKey::TrackArtist,
			ItemValue::Text(String::from("foo")),
		));
		tag.push_unchecked(TagItem::new(
			ItemKey::TrackArtist,
			ItemValue::Text(String::from("bar")),
		));
		tag.push_unchecked(TagItem::new(
			ItemKey::TrackArtist,
			ItemValue::Text(String::from("baz")),
		));

		let tag: Id3v2Tag = tag.into();
		assert_eq!(tag.artist().as_deref(), Some("foo/bar/baz"))
	}

	#[test]
	fn utf16_txxx_with_single_bom() {
		let _ = read_tag("tests/tags/assets/id3v2/issue_53.id3v24");
	}

	#[test]
	fn replaygain_tag_conversion() {
		let mut tag = Id3v2Tag::default();
		tag.insert(
			Frame::new(
				"TXXX",
				FrameValue::UserText(ExtendedTextFrame {
					encoding: TextEncoding::UTF8,
					description: String::from("REPLAYGAIN_ALBUM_GAIN"),
					content: String::from("-10.43 dB"),
				}),
				FrameFlags::default(),
			)
			.unwrap(),
		);

		let tag: Tag = tag.into();

		assert_eq!(tag.item_count(), 1);
		assert_eq!(
			tag.items[0],
			TagItem {
				item_key: ItemKey::ReplayGainAlbumGain,
				item_value: ItemValue::Text(String::from("-10.43 dB"))
			}
		);
	}

	#[test]
	fn multi_value_roundtrip() {
		let mut tag = Tag::new(TagType::Id3v2);
		// 1st: Multi-valued text frames
		tag.insert_text(ItemKey::TrackArtist, "TrackArtist 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::TrackArtist,
			ItemValue::Text("TrackArtist 2".to_owned()),
		));
		tag.insert_text(ItemKey::AlbumArtist, "AlbumArtist 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::AlbumArtist,
			ItemValue::Text("AlbumArtist 2".to_owned()),
		));
		tag.insert_text(ItemKey::TrackTitle, "TrackTitle 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::TrackTitle,
			ItemValue::Text("TrackTitle 2".to_owned()),
		));
		tag.insert_text(ItemKey::AlbumTitle, "AlbumTitle 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::AlbumTitle,
			ItemValue::Text("AlbumTitle 2".to_owned()),
		));
		tag.insert_text(ItemKey::ContentGroup, "ContentGroup 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::ContentGroup,
			ItemValue::Text("ContentGroup 2".to_owned()),
		));
		tag.insert_text(ItemKey::Genre, "Genre 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::Genre,
			ItemValue::Text("Genre 2".to_owned()),
		));
		tag.insert_text(ItemKey::Mood, "Mood 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::Mood,
			ItemValue::Text("Mood 2".to_owned()),
		));
		tag.insert_text(ItemKey::Composer, "Composer 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::Composer,
			ItemValue::Text("Composer 2".to_owned()),
		));
		tag.insert_text(ItemKey::Conductor, "Conductor 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::Conductor,
			ItemValue::Text("Conductor 2".to_owned()),
		));
		// 2nd: Multi-valued language frames
		tag.insert_text(ItemKey::Comment, "Comment 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::Comment,
			ItemValue::Text("Comment 2".to_owned()),
		));
		assert_eq!(20, tag.len());

		let id3v2 = Id3v2Tag::from(tag.clone());
		let (split_remainder, split_tag) = id3v2.split_tag();

		assert_eq!(0, split_remainder.0.len());
		assert_eq!(tag.len(), split_tag.len());
		// The ordering of items/frames matters, see above!
		// TODO: Replace with an unordered comparison.
		assert_eq!(tag.items, split_tag.items);
	}

	#[test]
	fn comments() {
		let mut tag = Id3v2Tag::default();
		let encoding = TextEncoding::Latin1;
		let flags = FrameFlags::default();
		let custom_descriptor = "lofty-rs";

		assert!(tag.comment().is_none());

		// Add an empty comment (which is a valid use case).
		tag.set_comment(String::new());
		assert_eq!(Some(Cow::Borrowed("")), tag.comment());

		// Insert a custom comment frame
		assert!(tag
			.frames
			.iter()
			.find_map(|frame| filter_comment_frame_by_description(frame, custom_descriptor))
			.is_none());
		tag.insert(
			Frame::new(
				COMMENT_FRAME_ID,
				FrameValue::Comment(CommentFrame {
					encoding,
					language: *b"eng",
					description: custom_descriptor.to_owned(),
					content: String::from("Qux comment"),
				}),
				flags,
			)
			.unwrap(),
		);
		// Verify that the regular comment still exists
		assert_eq!(Some(Cow::Borrowed("")), tag.comment());
		assert_eq!(1, tag.comments().count());

		tag.remove_comment();
		assert!(tag.comment().is_none());

		// Verify that the comment with the custom descriptor still exists
		assert!(tag
			.frames
			.iter()
			.find_map(|frame| filter_comment_frame_by_description(frame, custom_descriptor))
			.is_some());
	}

	#[test]
	fn txxx_wxxx_tag_conversion() {
		let txxx_frame = Frame::new(
			"TXXX",
			FrameValue::UserText(ExtendedTextFrame {
				encoding: TextEncoding::UTF8,
				description: String::from("FOO_TEXT_FRAME"),
				content: String::from("foo content"),
			}),
			FrameFlags::default(),
		)
		.unwrap();

		let wxxx_frame = Frame::new(
			"WXXX",
			FrameValue::UserUrl(ExtendedUrlFrame {
				encoding: TextEncoding::UTF8,
				description: String::from("BAR_URL_FRAME"),
				content: String::from("bar url"),
			}),
			FrameFlags::default(),
		)
		.unwrap();

		let mut tag = Id3v2Tag::default();

		tag.insert(txxx_frame.clone());
		tag.insert(wxxx_frame.clone());

		let tag: Tag = tag.into();

		assert_eq!(tag.item_count(), 2);
		let expected_items = [
			TagItem::new(
				ItemKey::Unknown(String::from("FOO_TEXT_FRAME")),
				ItemValue::Text(String::from("foo content")),
			),
			TagItem::new(
				ItemKey::Unknown(String::from("BAR_URL_FRAME")),
				ItemValue::Locator(String::from("bar url")),
			),
		];
		assert!(expected_items
			.iter()
			.zip(tag.items())
			.all(|(expected, actual)| expected == actual));

		let tag: Id3v2Tag = tag.into();

		assert_eq!(tag.frames.len(), 2);
		assert_eq!(&tag.frames, &[txxx_frame, wxxx_frame])
	}

	#[test]
	fn user_defined_frames_conversion() {
		let mut id3v2 = Id3v2Tag::default();
		id3v2.insert(
			Frame::new(
				"TXXX",
				FrameValue::UserText(ExtendedTextFrame {
					encoding: TextEncoding::UTF8,
					description: String::from("FOO_BAR"),
					content: String::from("foo content"),
				}),
				FrameFlags::default(),
			)
			.unwrap(),
		);

		let (split_remainder, split_tag) = id3v2.split_tag();
		assert_eq!(split_remainder.0.len(), 0);
		assert_eq!(split_tag.len(), 1);

		let id3v2 = split_remainder.merge_tag(split_tag);

		// Verify we properly convert user defined frames between Tag <-> ID3v2Tag round trips
		assert_eq!(
			id3v2.frames.first(),
			Some(&Frame {
				id: FrameId::Valid(Cow::Borrowed("TXXX")),
				value: FrameValue::UserText(ExtendedTextFrame {
					description: String::from("FOO_BAR"),
					encoding: TextEncoding::UTF8, // Not considered by PartialEq!
					content: String::new(),       // Not considered by PartialEq!
				}),
				flags: FrameFlags::default(),
			})
		);

		// Verify we properly convert user defined frames when writing a Tag, which has to convert
		// to the reference types.
		let (_remainder, tag) = id3v2.clone().split_tag();
		assert_eq!(tag.len(), 1);

		let mut content = Vec::new();
		tag.dump_to(&mut content).unwrap();
		assert!(!content.is_empty());

		// And verify we can reread the tag
		let mut reader = std::io::Cursor::new(&content[..]);

		let header = Id3v2Header::parse(&mut reader).unwrap();
		let reparsed =
			crate::id3::v2::read::parse_id3v2(&mut reader, header, ParsingMode::Strict).unwrap();

		assert_eq!(id3v2, reparsed);
	}

	#[test]
	fn set_track() {
		let mut id3v2 = Id3v2Tag::default();
		let track = 1;

		id3v2.set_track(track);

		assert_eq!(id3v2.track().unwrap(), track);
		assert!(id3v2.track_total().is_none());
	}

	#[test]
	fn set_track_total() {
		let mut id3v2 = Id3v2Tag::default();
		let track_total = 2;

		id3v2.set_track_total(track_total);

		assert_eq!(id3v2.track().unwrap(), DEFAULT_NUMBER_IN_PAIR);
		assert_eq!(id3v2.track_total().unwrap(), track_total);
	}

	#[test]
	fn set_track_and_track_total() {
		let mut id3v2 = Id3v2Tag::default();
		let track = 1;
		let track_total = 2;

		id3v2.set_track(track);
		id3v2.set_track_total(track_total);

		assert_eq!(id3v2.track().unwrap(), track);
		assert_eq!(id3v2.track_total().unwrap(), track_total);
	}

	#[test]
	fn set_track_total_and_track() {
		let mut id3v2 = Id3v2Tag::default();
		let track_total = 2;
		let track = 1;

		id3v2.set_track_total(track_total);
		id3v2.set_track(track);

		assert_eq!(id3v2.track_total().unwrap(), track_total);
		assert_eq!(id3v2.track().unwrap(), track);
	}

	#[test]
	fn set_disk() {
		let mut id3v2 = Id3v2Tag::default();
		let disk = 1;

		id3v2.set_disk(disk);

		assert_eq!(id3v2.disk().unwrap(), disk);
		assert!(id3v2.disk_total().is_none());
	}

	#[test]
	fn set_disk_total() {
		let mut id3v2 = Id3v2Tag::default();
		let disk_total = 2;

		id3v2.set_disk_total(disk_total);

		assert_eq!(id3v2.disk().unwrap(), DEFAULT_NUMBER_IN_PAIR);
		assert_eq!(id3v2.disk_total().unwrap(), disk_total);
	}

	#[test]
	fn set_disk_and_disk_total() {
		let mut id3v2 = Id3v2Tag::default();
		let disk = 1;
		let disk_total = 2;

		id3v2.set_disk(disk);
		id3v2.set_disk_total(disk_total);

		assert_eq!(id3v2.disk().unwrap(), disk);
		assert_eq!(id3v2.disk_total().unwrap(), disk_total);
	}

	#[test]
	fn set_disk_total_and_disk() {
		let mut id3v2 = Id3v2Tag::default();
		let disk_total = 2;
		let disk = 1;

		id3v2.set_disk_total(disk_total);
		id3v2.set_disk(disk);

		assert_eq!(id3v2.disk_total().unwrap(), disk_total);
		assert_eq!(id3v2.disk().unwrap(), disk);
	}

	#[test]
	fn track_number_tag_to_id3v2() {
		use crate::traits::Accessor;
		let track_number = 1;

		let mut tag = Tag::new(TagType::Id3v2);

		tag.push(TagItem::new(
			ItemKey::TrackNumber,
			ItemValue::Text(track_number.to_string()),
		));

		let tag: Id3v2Tag = tag.into();

		assert_eq!(tag.track().unwrap(), track_number);
		assert!(tag.track_total().is_none());
	}

	#[test]
	fn track_total_tag_to_id3v2() {
		use crate::traits::Accessor;
		let track_total = 2;

		let mut tag = Tag::new(TagType::Id3v2);

		tag.push(TagItem::new(
			ItemKey::TrackTotal,
			ItemValue::Text(track_total.to_string()),
		));

		let tag: Id3v2Tag = tag.into();

		assert_eq!(tag.track().unwrap(), DEFAULT_NUMBER_IN_PAIR);
		assert_eq!(tag.track_total().unwrap(), track_total);
	}

	#[test]
	fn track_number_and_track_total_tag_to_id3v2() {
		use crate::traits::Accessor;
		let track_number = 1;
		let track_total = 2;

		let mut tag = Tag::new(TagType::Id3v2);

		tag.push(TagItem::new(
			ItemKey::TrackNumber,
			ItemValue::Text(track_number.to_string()),
		));

		tag.push(TagItem::new(
			ItemKey::TrackTotal,
			ItemValue::Text(track_total.to_string()),
		));

		let tag: Id3v2Tag = tag.into();

		assert_eq!(tag.track().unwrap(), track_number);
		assert_eq!(tag.track_total().unwrap(), track_total);
	}

	#[test]
	fn disk_number_tag_to_id3v2() {
		use crate::traits::Accessor;
		let disk_number = 1;

		let mut tag = Tag::new(TagType::Id3v2);

		tag.push(TagItem::new(
			ItemKey::DiscNumber,
			ItemValue::Text(disk_number.to_string()),
		));

		let tag: Id3v2Tag = tag.into();

		assert_eq!(tag.disk().unwrap(), disk_number);
		assert!(tag.disk_total().is_none());
	}

	#[test]
	fn disk_total_tag_to_id3v2() {
		use crate::traits::Accessor;
		let disk_total = 2;

		let mut tag = Tag::new(TagType::Id3v2);

		tag.push(TagItem::new(
			ItemKey::DiscTotal,
			ItemValue::Text(disk_total.to_string()),
		));

		let tag: Id3v2Tag = tag.into();

		assert_eq!(tag.disk().unwrap(), DEFAULT_NUMBER_IN_PAIR);
		assert_eq!(tag.disk_total().unwrap(), disk_total);
	}

	#[test]
	fn disk_number_and_disk_total_tag_to_id3v2() {
		use crate::traits::Accessor;
		let disk_number = 1;
		let disk_total = 2;

		let mut tag = Tag::new(TagType::Id3v2);

		tag.push(TagItem::new(
			ItemKey::DiscNumber,
			ItemValue::Text(disk_number.to_string()),
		));

		tag.push(TagItem::new(
			ItemKey::DiscTotal,
			ItemValue::Text(disk_total.to_string()),
		));

		let tag: Id3v2Tag = tag.into();

		assert_eq!(tag.disk().unwrap(), disk_number);
		assert_eq!(tag.disk_total().unwrap(), disk_total);
	}

	fn create_tag_with_trck_and_tpos_frame(content: &'static str) -> Tag {
		fn insert_frame(id: &'static str, content: &'static str, tag: &mut Id3v2Tag) {
			tag.insert(new_text_frame(
				FrameId::Valid(Cow::Borrowed(id)),
				content.to_string(),
				FrameFlags::default(),
			));
		}

		let mut tag = Id3v2Tag::default();

		insert_frame("TRCK", content, &mut tag);
		insert_frame("TPOS", content, &mut tag);

		tag.into()
	}

	#[test]
	fn valid_trck_and_tpos_frame() {
		fn assert_valid(content: &'static str, number: Option<u32>, total: Option<u32>) {
			let tag = create_tag_with_trck_and_tpos_frame(content);

			assert_eq!(tag.track(), number);
			assert_eq!(tag.track_total(), total);
			assert_eq!(tag.disk(), number);
			assert_eq!(tag.disk_total(), total);
		}

		assert_valid("0", Some(0), None);
		assert_valid("1", Some(1), None);
		assert_valid("0/0", Some(0), Some(0));
		assert_valid("1/2", Some(1), Some(2));
		assert_valid("010/011", Some(10), Some(11));
		assert_valid(" 1/2 ", Some(1), Some(2));
		assert_valid("1 / 2", Some(1), Some(2));
	}

	#[test]
	fn invalid_trck_and_tpos_frame() {
		fn assert_invalid(content: &'static str) {
			let tag = create_tag_with_trck_and_tpos_frame(content);

			assert!(tag.track().is_none());
			assert!(tag.track_total().is_none());
			assert!(tag.disk().is_none());
			assert!(tag.disk_total().is_none());
		}

		assert_invalid("");
		assert_invalid(" ");
		assert_invalid("/");
		assert_invalid("/1");
		assert_invalid("1/");
		assert_invalid("a/b");
		assert_invalid("1/2/3");
		assert_invalid("1//2");
		assert_invalid("0x1/0x2");
	}

	#[test]
	fn ufid_frame_with_musicbrainz_record_id() {
		let mut id3v2 = Id3v2Tag::default();
		let unknown_ufid_frame = UniqueFileIdentifierFrame {
			owner: "other".to_owned(),
			identifier: b"0123456789".to_vec(),
		};
		id3v2.insert(
			Frame::new(
				"UFID",
				FrameValue::UniqueFileIdentifier(unknown_ufid_frame.clone()),
				FrameFlags::default(),
			)
			.unwrap(),
		);
		let musicbrainz_recording_id = b"189002e7-3285-4e2e-92a3-7f6c30d407a2";
		let musicbrainz_recording_id_frame = UniqueFileIdentifierFrame {
			owner: MUSICBRAINZ_UFID_OWNER.to_owned(),
			identifier: musicbrainz_recording_id.to_vec(),
		};
		id3v2.insert(
			Frame::new(
				"UFID",
				FrameValue::UniqueFileIdentifier(musicbrainz_recording_id_frame.clone()),
				FrameFlags::default(),
			)
			.unwrap(),
		);
		assert_eq!(2, id3v2.len());

		let (split_remainder, split_tag) = id3v2.split_tag();
		assert_eq!(split_remainder.0.len(), 1);
		assert_eq!(split_tag.len(), 1);
		assert_eq!(
			ItemValue::Text(String::from_utf8(musicbrainz_recording_id.to_vec()).unwrap()),
			*split_tag
				.get_items(&ItemKey::MusicBrainzRecordingId)
				.next()
				.unwrap()
				.value()
		);

		let id3v2 = split_remainder.merge_tag(split_tag);
		assert_eq!(2, id3v2.len());
		match &id3v2.frames[..] {
			[Frame {
				id: _,
				value:
					FrameValue::UniqueFileIdentifier(UniqueFileIdentifierFrame {
						owner: first_owner,
						identifier: first_identifier,
					}),
				flags: _,
			}, Frame {
				id: _,
				value:
					FrameValue::UniqueFileIdentifier(UniqueFileIdentifierFrame {
						owner: second_owner,
						identifier: second_identifier,
					}),
				flags: _,
			}] => {
				assert_eq!(&unknown_ufid_frame.owner, first_owner);
				assert_eq!(&unknown_ufid_frame.identifier, first_identifier);
				assert_eq!(&musicbrainz_recording_id_frame.owner, second_owner);
				assert_eq!(
					&musicbrainz_recording_id_frame.identifier,
					second_identifier
				);
			},
			_ => unreachable!(),
		}
	}

	#[test]
	fn get_set_user_defined_text() {
		let description = String::from("FOO_BAR");
		let content = String::from("Baz!\0Qux!");
		let description2 = String::from("FOO_BAR_2");
		let content2 = String::new();

		let mut id3v2 = Id3v2Tag::default();
		let txxx_frame = Frame::new(
			"TXXX",
			ExtendedTextFrame {
				encoding: TextEncoding::UTF8,
				description: description.clone(),
				content: content.clone(),
			},
			FrameFlags::default(),
		)
		.unwrap();

		id3v2.insert(txxx_frame.clone());

		// Insert another to verify we can search through multiple
		let txxx_frame2 = Frame::new(
			"TXXX",
			ExtendedTextFrame {
				encoding: TextEncoding::UTF8,
				description: description2.clone(),
				content: content2.clone(),
			},
			FrameFlags::default(),
		)
		.unwrap();
		id3v2.insert(txxx_frame2);

		// We cannot get user defined texts through `get_text`
		assert!(id3v2
			.get_text(&FrameId::Valid(Cow::Borrowed("TXXX")))
			.is_none());

		assert_eq!(id3v2.get_user_text(description.as_str()), Some(&*content));

		// Wipe the tag
		id3v2.clear();

		// Same thing process as above, using simplified setter
		assert!(id3v2
			.insert_user_text(description.clone(), content.clone())
			.is_none());
		assert!(id3v2
			.insert_user_text(description2.clone(), content2.clone())
			.is_none());
		assert_eq!(id3v2.get_user_text(description.as_str()), Some(&*content));

		// Remove one frame
		assert!(id3v2.remove_user_text(&description).is_some());
		assert!(!id3v2.is_empty());

		// Now clear the remaining item
		assert!(id3v2.remove_user_text(&description2).is_some());
		assert!(id3v2.is_empty());
	}

	#[test]
	fn read_multiple_composers_should_not_fail_with_bad_frame_length() {
		// Issue #255
		let tag = read_tag("tests/tags/assets/id3v2/multiple_composers.id3v24");
		let mut composers = tag
			.get_texts(&FrameId::Valid(Cow::Borrowed("TCOM")))
			.unwrap();

		assert_eq!(composers.next(), Some("A"));
		assert_eq!(composers.next(), Some("B"));
		assert_eq!(composers.next(), None)
	}

	#[test]
	fn trim_end_nulls_when_reading_frame_content() {
		// Issue #273
		// Tag written by mid3v2. All frames contain null-terminated UTF-8 text
		let tag = read_tag("tests/tags/assets/id3v2/trailing_nulls.id3v24");

		// Verify that each different frame type no longer has null terminator
		let artist = tag.get_text(&FrameId::Valid(Cow::Borrowed("TPE1")));
		assert_eq!(artist.unwrap(), "Artist");

		let writer = tag.get_user_text("Writer");
		assert_eq!(writer.unwrap(), "Writer");

		let lyrics = &tag.unsync_text().next().unwrap().content;
		assert_eq!(lyrics, "Lyrics to the song");

		let comment = tag.comment().unwrap();
		assert_eq!(comment, "Comment");

		let url_frame = tag.get(&FrameId::Valid(Cow::Borrowed("WXXX"))).unwrap();
		let FrameValue::UserUrl(url) = &url_frame.value else {
			panic!("Expected a UserUrl")
		};
		assert_eq!(url.content, "https://www.myfanpage.com");
	}

	fn id3v2_tag_with_genre(value: &str) -> Id3v2Tag {
		let mut tag = Id3v2Tag::default();
		let frame = new_text_frame(GENRE_ID, String::from(value), FrameFlags::default());
		tag.insert(frame);
		tag
	}

	#[test]
	fn genre_text() {
		let tag = id3v2_tag_with_genre("Dream Pop");
		assert_eq!(tag.genre(), Some(Cow::Borrowed("Dream Pop")));
	}
	#[test]
	fn genre_id_brackets() {
		let tag = id3v2_tag_with_genre("(21)");
		assert_eq!(tag.genre(), Some(Cow::Borrowed("Ska")));
	}

	#[test]
	fn genre_id_numeric() {
		let tag = id3v2_tag_with_genre("21");
		assert_eq!(tag.genre(), Some(Cow::Borrowed("Ska")));
	}

	#[test]
	fn genre_id_multiple_joined() {
		let tag = id3v2_tag_with_genre("(51)(39)");
		assert_eq!(
			tag.genre(),
			Some(Cow::Borrowed("Techno-Industrial / Noise"))
		);
	}

	#[test]
	fn genres_id_multiple() {
		let tag = id3v2_tag_with_genre("(51)(39)");
		let mut genres = tag.genres().unwrap();
		assert_eq!(genres.next(), Some("Techno-Industrial"));
		assert_eq!(genres.next(), Some("Noise"));
		assert_eq!(genres.next(), None);
	}

	#[test]
	fn genres_id_multiple_into_tag() {
		let id3v2 = id3v2_tag_with_genre("(51)(39)");
		let tag: Tag = id3v2.into();
		let mut genres = tag.get_strings(&ItemKey::Genre);
		assert_eq!(genres.next(), Some("Techno-Industrial"));
		assert_eq!(genres.next(), Some("Noise"));
		assert_eq!(genres.next(), None);
	}

	#[test]
	fn genres_null_separated() {
		let tag = id3v2_tag_with_genre("Samba-rock\0MPB\0Funk");
		let mut genres = tag.genres().unwrap();
		assert_eq!(genres.next(), Some("Samba-rock"));
		assert_eq!(genres.next(), Some("MPB"));
		assert_eq!(genres.next(), Some("Funk"));
		assert_eq!(genres.next(), None);
	}

	#[test]
	fn genres_id_textual_refinement() {
		let tag = id3v2_tag_with_genre("(4)Eurodisco");
		let mut genres = tag.genres().unwrap();
		assert_eq!(genres.next(), Some("Disco"));
		assert_eq!(genres.next(), Some("Eurodisco"));
		assert_eq!(genres.next(), None);
	}

	#[test]
	fn genres_id_bracketed_refinement() {
		let tag = id3v2_tag_with_genre("(26)(55)((I think...)");
		let mut genres = tag.genres().unwrap();
		assert_eq!(genres.next(), Some("Ambient"));
		assert_eq!(genres.next(), Some("Dream"));
		assert_eq!(genres.next(), Some("(I think...)"));
		assert_eq!(genres.next(), None);
	}

	#[test]
	fn genres_id_remix_cover() {
		let tag = id3v2_tag_with_genre("(0)(RX)(CR)");
		let mut genres = tag.genres().unwrap();
		assert_eq!(genres.next(), Some("Blues"));
		assert_eq!(genres.next(), Some("Remix"));
		assert_eq!(genres.next(), Some("Cover"));
		assert_eq!(genres.next(), None);
	}

	#[test]
	fn tipl_round_trip() {
		let mut tag = Id3v2Tag::default();
		let mut tipl = KeyValueFrame {
			encoding: TextEncoding::UTF8,
			key_value_pairs: Vec::new(),
		};

		// Add all supported keys
		for (_, key) in TIPL_MAPPINGS {
			tipl.key_value_pairs
				.push((String::from(*key), String::from("Serial-ATA")));
		}

		// Add one unsupported key
		tipl.key_value_pairs
			.push((String::from("Foo"), String::from("Bar")));

		tag.insert(
			Frame::new(
				"TIPL",
				FrameValue::KeyValue(tipl.clone()),
				FrameFlags::default(),
			)
			.unwrap(),
		);

		let (split_remainder, split_tag) = tag.split_tag();
		assert_eq!(split_remainder.0.len(), 1); // "Foo" is not supported
		assert_eq!(split_tag.len(), TIPL_MAPPINGS.len()); // All supported keys are present

		for (item_key, _) in TIPL_MAPPINGS {
			assert_eq!(
				split_tag
					.get(item_key)
					.map(TagItem::value)
					.and_then(ItemValue::text),
				Some("Serial-ATA")
			);
		}

		let mut id3v2 = split_remainder.merge_tag(split_tag);
		assert_eq!(id3v2.frames.len(), 1);
		match &mut id3v2.frames[..] {
			[Frame {
				id: _,
				value: FrameValue::KeyValue(tipl2),
				flags: _,
			}] => {
				// Order will not be the same, so we have to sort first
				tipl.key_value_pairs.sort();
				tipl2.key_value_pairs.sort();
				assert_eq!(tipl, *tipl2);
			},
			_ => unreachable!(),
		}
	}
}
