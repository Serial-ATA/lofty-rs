use crate::config::WriteOptions;
use crate::error::{LoftyError, Result};
use crate::id3::v1::constants::GENRES;
use crate::tag::items::Timestamp;
use crate::tag::{Accessor, ItemKey, ItemValue, MergeTag, SplitTag, Tag, TagExt, TagItem, TagType};
use crate::util::io::{FileLike, Length, Truncate};

use std::borrow::Cow;
use std::io::Write;
use std::path::Path;

use lofty_attr::tag;

macro_rules! impl_accessor {
	($($name:ident,)+) => {
		paste::paste! {
			$(
				fn $name(&self) -> Option<Cow<'_, str>> {
					if let Some(item) = self.$name.as_deref() {
						return Some(Cow::Borrowed(item));
					}

					None
				}

				fn [<set_ $name>](&mut self, value: String) {
					self.$name = Some(value)
				}

				fn [<remove_ $name>](&mut self) {
					self.$name = None
				}
			)+
		}
	}
}

/// ID3v1 is a severely limited format, with each field
/// being incredibly small in size. All fields have been
/// commented with their maximum sizes and any other additional
/// restrictions.
///
/// Attempting to write a field greater than the maximum size
/// will **not** error, it will just be shrunk.
///
/// ## Conversions
///
/// ### To `Tag`
///
/// All fields can be translated to a `TagItem`:
///
/// * `title` -> [`ItemKey::TrackTitle`]
/// * `artist` -> [`ItemKey::TrackArtist`]
/// * `album` -> [`ItemKey::AlbumTitle`]
/// * `year` -> [`ItemKey::Year`]
/// * `comment` -> [`ItemKey::Comment`]
/// * `track_number` -> [`ItemKey::TrackNumber`]
/// * `genre` -> [`ItemKey::Genre`] (As long as the genre is a valid index into [`GENRES`])
/// 	* Note that [`ItemKey::Genre`] will contain the *string* at [`GENRES`]\[index\], not the index.
///
/// ### From `Tag`
///
/// #### Items
///
/// All of the [`ItemKey`]s referenced in the conversion to [`Tag`] will be checked.
///
/// The values will be used as-is, with two exceptions:
///
/// * [`ItemKey::TrackNumber`] - Will only be used if the value can be parsed as a `u8`
/// * [`ItemKey::Genre`] - Will only be used if:
/// 	* [`GENRES`] contains the string
/// 	* **OR** The [`ItemValue`](crate::ItemValue) can be parsed into a `u8` ***and*** it is a valid index into [`GENRES`]
///
/// #### Pictures
///
/// Pictures will be discarded, as they aren't supported in this format.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
#[tag(
	description = "An ID3v1 tag",
	supported_formats(Aac, Ape, Mpeg, WavPack, read_only(Mpc))
)]
pub struct Id3v1Tag {
	/// Track title, 30 bytes max
	pub title: Option<String>,
	/// Track artist, 30 bytes max
	pub artist: Option<String>,
	/// Album title, 30 bytes max
	pub album: Option<String>,
	/// Release year (max 9999)
	pub year: Option<u16>,
	/// A short comment
	///
	/// The number of bytes differs between versions, but not much.
	/// A V1 tag may have been read, which limits this field to 30 bytes.
	/// A V1.1 tag, however, only has 28 bytes available.
	///
	/// **Lofty** will *always* write a V1.1 tag.
	pub comment: Option<String>,
	/// The track number, 1 byte max
	///
	/// Issues:
	///
	/// * The track number **cannot** be 0. Many readers, including Lofty,
	/// look for a null byte at the end of the comment to differentiate
	/// between V1 and V1.1.
	/// * A V1 tag may have been read, which does *not* have a track number.
	pub track_number: Option<u8>,
	/// The track's genre, 1 byte max
	///
	/// ID3v1 has a predefined set of genres, see [`GENRES`](crate::id3::v1::GENRES).
	/// This byte should be an index to a genre.
	pub genre: Option<u8>,
}

impl Id3v1Tag {
	/// Create a new empty `ID3v1Tag`
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::id3::v1::Id3v1Tag;
	/// use lofty::tag::TagExt;
	///
	/// let id3v1_tag = Id3v1Tag::new();
	/// assert!(id3v1_tag.is_empty());
	/// ```
	pub fn new() -> Self {
		Self::default()
	}
}

impl Accessor for Id3v1Tag {
	impl_accessor!(title, artist, album,);

	fn genre(&self) -> Option<Cow<'_, str>> {
		if let Some(g) = self.genre {
			let g = g as usize;

			if g < GENRES.len() {
				return Some(Cow::Borrowed(GENRES[g]));
			}
		}

		None
	}

	fn set_genre(&mut self, genre: String) {
		let g_str = genre.as_str();

		for (i, g) in GENRES.iter().enumerate() {
			if g.eq_ignore_ascii_case(g_str) {
				self.genre = Some(i as u8);
				break;
			}
		}
	}

	fn remove_genre(&mut self) {
		self.genre = None
	}

	fn track(&self) -> Option<u32> {
		self.track_number.map(u32::from)
	}

	fn set_track(&mut self, value: u32) {
		self.track_number = Some(value as u8);
	}

	fn remove_track(&mut self) {
		self.track_number = None;
	}

	fn comment(&self) -> Option<Cow<'_, str>> {
		self.comment.as_deref().map(Cow::Borrowed)
	}

	fn set_comment(&mut self, value: String) {
		let mut resized = String::with_capacity(28);
		for c in value.chars() {
			if resized.len() + c.len_utf8() > 28 {
				break;
			}

			resized.push(c);
		}

		self.comment = Some(resized);
	}

	fn remove_comment(&mut self) {
		self.comment = None;
	}

	fn date(&self) -> Option<Timestamp> {
		self.year.map(|year| Timestamp {
			year,
			..Default::default()
		})
	}

	fn set_date(&mut self, value: Timestamp) {
		self.year = Some(value.year);
	}

	fn remove_date(&mut self) {
		self.year = None;
	}
}

impl TagExt for Id3v1Tag {
	type Err = LoftyError;
	type RefKey<'a> = &'a ItemKey;

	#[inline]
	fn tag_type(&self) -> TagType {
		TagType::Id3v1
	}

	fn len(&self) -> usize {
		usize::from(self.title.is_some())
			+ usize::from(self.artist.is_some())
			+ usize::from(self.album.is_some())
			+ usize::from(self.year.is_some())
			+ usize::from(self.comment.is_some())
			+ usize::from(self.track_number.is_some())
			+ usize::from(self.genre.is_some())
	}

	fn contains<'a>(&'a self, key: Self::RefKey<'a>) -> bool {
		match key {
			ItemKey::TrackTitle => self.title.is_some(),
			ItemKey::AlbumTitle => self.album.is_some(),
			ItemKey::TrackArtist => self.artist.is_some(),
			ItemKey::TrackNumber => self.track_number.is_some(),
			ItemKey::Year => self.year.is_some(),
			ItemKey::Genre => self.genre.is_some(),
			ItemKey::Comment => self.comment.is_some(),
			_ => false,
		}
	}

	fn is_empty(&self) -> bool {
		self.title.is_none()
			&& self.artist.is_none()
			&& self.album.is_none()
			&& self.year.is_none()
			&& self.comment.is_none()
			&& self.track_number.is_none()
			&& self.genre.is_none()
	}

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
		Into::<Id3v1TagRef<'_>>::into(self).write_to(file, write_options)
	}

	/// Dumps the tag to a writer
	///
	/// # Errors
	///
	/// * [`std::io::Error`]
	fn dump_to<W: Write>(
		&self,
		writer: &mut W,
		write_options: WriteOptions,
	) -> std::result::Result<(), Self::Err> {
		Into::<Id3v1TagRef<'_>>::into(self).dump_to(writer, write_options)
	}

	fn remove_from_path<P: AsRef<Path>>(&self, path: P) -> std::result::Result<(), Self::Err> {
		TagType::Id3v1.remove_from_path(path)
	}

	fn remove_from<F>(&self, file: &mut F) -> std::result::Result<(), Self::Err>
	where
		F: FileLike,
		LoftyError: From<<F as Truncate>::Error>,
		LoftyError: From<<F as Length>::Error>,
	{
		TagType::Id3v1.remove_from(file)
	}

	fn clear(&mut self) {
		*self = Self::default();
	}
}

#[derive(Debug, Clone, Default)]
pub struct SplitTagRemainder;

impl SplitTag for Id3v1Tag {
	type Remainder = SplitTagRemainder;

	fn split_tag(mut self) -> (Self::Remainder, Tag) {
		let mut tag = Tag::new(TagType::Id3v1);

		self.title
			.take()
			.map(|t| tag.insert_text(ItemKey::TrackTitle, t));
		self.artist
			.take()
			.map(|a| tag.insert_text(ItemKey::TrackArtist, a));
		self.album
			.take()
			.map(|a| tag.insert_text(ItemKey::AlbumTitle, a));
		self.year
			.take()
			.map(|y| tag.insert_text(ItemKey::Year, y.to_string()));
		self.comment
			.take()
			.map(|c| tag.insert_text(ItemKey::Comment, c));

		if let Some(t) = self.track_number.take() {
			tag.items.push(TagItem::new(
				ItemKey::TrackNumber,
				ItemValue::Text(t.to_string()),
			))
		}

		if let Some(genre_index) = self.genre.take() {
			if let Some(genre) = GENRES.get(genre_index as usize) {
				tag.insert_text(ItemKey::Genre, (*genre).to_string());
			}
		}

		(SplitTagRemainder, tag)
	}
}

impl MergeTag for SplitTagRemainder {
	type Merged = Id3v1Tag;

	fn merge_tag(self, tag: Tag) -> Self::Merged {
		tag.into()
	}
}

impl From<Id3v1Tag> for Tag {
	fn from(input: Id3v1Tag) -> Self {
		input.split_tag().1
	}
}

impl From<Tag> for Id3v1Tag {
	fn from(mut input: Tag) -> Self {
		let title = input.take_strings(ItemKey::TrackTitle).next();
		let artist = input.take_strings(ItemKey::TrackArtist).next();
		let album = input.take_strings(ItemKey::AlbumTitle).next();
		let year = input
			.get_string(ItemKey::Year)
			.and_then(|year| year.parse().ok())
			.or_else(|| input.date().map(|y| y.year));
		let comment = input.take_strings(ItemKey::Comment).next();
		Self {
			title,
			artist,
			album,
			year,
			comment,
			track_number: input
				.get_string(ItemKey::TrackNumber)
				.map(|g| g.parse::<u8>().ok())
				.and_then(|g| g),
			genre: input
				.get_string(ItemKey::Genre)
				.map(|g| {
					GENRES
						.iter()
						.position(|v| v == &g)
						.map_or_else(|| g.parse::<u8>().ok(), |p| Some(p as u8))
				})
				.and_then(|g| g),
		}
	}
}

pub(crate) struct Id3v1TagRef<'a> {
	pub title: Option<&'a str>,
	pub artist: Option<&'a str>,
	pub album: Option<&'a str>,
	pub year: Option<u16>,
	pub comment: Option<&'a str>,
	pub track_number: Option<u8>,
	pub genre: Option<u8>,
}

impl<'a> Into<Id3v1TagRef<'a>> for &'a Id3v1Tag {
	fn into(self) -> Id3v1TagRef<'a> {
		Id3v1TagRef {
			title: self.title.as_deref(),
			artist: self.artist.as_deref(),
			album: self.album.as_deref(),
			year: self.year,
			comment: self.comment.as_deref(),
			track_number: self.track_number,
			genre: self.genre,
		}
	}
}

impl<'a> Into<Id3v1TagRef<'a>> for &'a Tag {
	fn into(self) -> Id3v1TagRef<'a> {
		Id3v1TagRef {
			title: self.get_string(ItemKey::TrackTitle),
			artist: self.get_string(ItemKey::TrackArtist),
			album: self.get_string(ItemKey::AlbumTitle),
			year: self
				.get_string(ItemKey::Year)
				.and_then(|year| year.parse().ok())
				.or_else(|| self.date().map(|date| date.year)),
			comment: self.get_string(ItemKey::Comment),
			track_number: self
				.get_string(ItemKey::TrackNumber)
				.map(|g| g.parse::<u8>().ok())
				.and_then(|g| g),
			genre: self
				.get_string(ItemKey::Genre)
				.map(|g| {
					GENRES
						.iter()
						.position(|v| v == &g)
						.map_or_else(|| g.parse::<u8>().ok(), |p| Some(p as u8))
				})
				.and_then(|g| g),
		}
	}
}

impl Id3v1TagRef<'_> {
	pub(super) fn is_empty(&self) -> bool {
		self.title.is_none()
			&& self.artist.is_none()
			&& self.album.is_none()
			&& self.year.is_none()
			&& self.comment.is_none()
			&& self.track_number.is_none()
			&& self.genre.is_none()
	}

	pub(crate) fn write_to<F>(&self, file: &mut F, write_options: WriteOptions) -> Result<()>
	where
		F: FileLike,
		LoftyError: From<<F as Truncate>::Error>,
		LoftyError: From<<F as Length>::Error>,
	{
		super::write::write_id3v1(file, self, write_options)
	}

	pub(crate) fn dump_to<W: Write>(
		&mut self,
		writer: &mut W,
		write_options: WriteOptions,
	) -> Result<()> {
		let temp = super::write::encode(self, write_options)?;
		writer.write_all(&temp)?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use crate::config::{ParsingMode, WriteOptions};
	use crate::id3::v1::Id3v1Tag;
	use crate::prelude::*;
	use crate::tag::items::Timestamp;
	use crate::tag::{Tag, TagType};

	#[test_log::test]
	fn parse_id3v1() {
		let expected_tag = Id3v1Tag {
			title: Some(String::from("Foo title")),
			artist: Some(String::from("Bar artist")),
			album: Some(String::from("Baz album")),
			year: Some(1984),
			comment: Some(String::from("Qux comment")),
			track_number: Some(1),
			genre: Some(32),
		};

		let tag = crate::tag::utils::test_utils::read_path("tests/tags/assets/test.id3v1");
		let parsed_tag = Id3v1Tag::parse(tag.try_into().unwrap(), ParsingMode::Strict).unwrap();

		assert_eq!(expected_tag, parsed_tag);
	}

	#[test_log::test]
	fn id3v1_re_read() {
		let tag = crate::tag::utils::test_utils::read_path("tests/tags/assets/test.id3v1");
		let parsed_tag = Id3v1Tag::parse(tag.try_into().unwrap(), ParsingMode::Strict).unwrap();

		let mut writer = Vec::new();
		parsed_tag
			.dump_to(&mut writer, WriteOptions::default())
			.unwrap();

		let temp_parsed_tag =
			Id3v1Tag::parse(writer.try_into().unwrap(), ParsingMode::Strict).unwrap();

		assert_eq!(parsed_tag, temp_parsed_tag);
	}

	#[test_log::test]
	fn id3v1_to_tag() {
		let tag_bytes = crate::tag::utils::test_utils::read_path("tests/tags/assets/test.id3v1");
		let id3v1 = Id3v1Tag::parse(tag_bytes.try_into().unwrap(), ParsingMode::Strict).unwrap();

		let tag: Tag = id3v1.into();

		crate::tag::utils::test_utils::verify_tag(&tag, true, true);
	}

	#[test_log::test]
	fn tag_to_id3v1() {
		let tag = crate::tag::utils::test_utils::create_tag(TagType::Id3v1);

		let id3v1_tag: Id3v1Tag = tag.into();

		assert_eq!(id3v1_tag.title.as_deref(), Some("Foo title"));
		assert_eq!(id3v1_tag.artist.as_deref(), Some("Bar artist"));
		assert_eq!(id3v1_tag.album.as_deref(), Some("Baz album"));
		assert_eq!(id3v1_tag.comment.as_deref(), Some("Qux comment"));
		assert_eq!(id3v1_tag.track_number, Some(1));
		assert_eq!(id3v1_tag.genre, Some(32));
	}

	#[test_log::test]
	fn year_roundtrip() {
		// via set_date(), which uses `ItemKey::RecordingDate`

		let mut tag = Tag::new(TagType::Id3v1);
		tag.set_date(Timestamp {
			year: 1999,
			month: Some(10),
			day: Some(11),
			hour: Some(12),
			minute: Some(13),
			second: Some(14),
		});

		let id3v1_tag: Id3v1Tag = tag.into();

		assert_eq!(id3v1_tag.year, Some(1999));
		assert_eq!(
			id3v1_tag.date(),
			Some(Timestamp {
				year: 1999,
				..Timestamp::default()
			})
		);

		// via `ItemKey::Year`

		let mut tag = Tag::new(TagType::Id3v1);
		tag.insert_text(ItemKey::Year, 1999u16.to_string());

		let id3v1_tag: Id3v1Tag = tag.into();

		assert_eq!(id3v1_tag.year, Some(1999));
		assert_eq!(
			id3v1_tag.date(),
			Some(Timestamp {
				year: 1999,
				..Timestamp::default()
			})
		);
	}

	#[test_log::test]
	fn lossy_encodings() {
		let mut tag = Tag::new(TagType::Id3v1);
		tag.set_artist(String::from("lфfty"));

		// Lossy encoding should pass
		let id3v1_tag: Id3v1Tag = tag.into();
		let mut bytes = Vec::new();
		id3v1_tag
			.dump_to(&mut bytes, WriteOptions::new().lossy_text_encoding(true))
			.unwrap();

		let id3v1 = Id3v1Tag::parse(bytes.try_into().unwrap(), ParsingMode::BestAttempt).unwrap();
		assert_eq!(id3v1.artist.as_deref(), Some("l?fty"));

		// And should fail when disabled
		id3v1_tag
			.dump_to(
				&mut Vec::new(),
				WriteOptions::new().lossy_text_encoding(false),
			)
			.unwrap_err();
	}
}
