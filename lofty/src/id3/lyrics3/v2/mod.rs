//! Lyrics3 v2 items
//!
//! # Lyrics3 notes
//!
//! See also: [`Lyrics3v2Tag`]

pub(crate) mod constants;
pub(crate) mod errors;
mod read;
#[cfg(test)]
mod tests;
mod write;

use crate::config::{WriteOptions, global_options};
use crate::error::{FileEncodingError, TagEncodingError};
use crate::io::{FileLike, VerifiedFile};
use crate::prelude::ItemKey;
use crate::tag::items::Timestamp;
use crate::tag::{Accessor, CompanionTag, MergeTag, SplitTag, Tag, TagExt, TagType, TagWriteExt};
use errors::Lyrics3v2EncodingError;

use std::io::Write;

use lofty_attr::tag;

macro_rules! impl_accessor {
	($($field:ident => $name:ident,)+) => {
		paste::paste! {
			$(
				fn $name(&self) -> Option<std::borrow::Cow<'_, str>> {
					if let Some(item) = self.$field.as_deref() {
						return Some(std::borrow::Cow::Borrowed(item));
					}

					None
				}

				fn [<set_ $name>](&mut self, value: String) {
					self.$field = Some(value)
				}

				fn [<remove_ $name>](&mut self) {
					self.$field = None
				}
			)+
		}
	}
}

/// Flags for an [`Lyrics3v2Tag`]
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct Lyrics3v2TagFlags {
	/// Whether the `lyrics` field contains any timestamps
	pub synchronized_lyrics: bool,
	/// Whether the track should be inhibited from random selection
	pub random_selection: bool,
}

/// An image file link in a [`Lyrics3v2Tag`]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Image {
	/// The filename of the image
	///
	/// Technically an arbitrary text field, but it **should** be a JPG or BMP image path in one of
	/// the following formats:
	///
	/// * Filename only (i.e. `cover.jpg`)
	/// * Relative Path + Filename (i.e. `images\cover.jpg`)
	/// * Full path + Filename (i.e. `c:\images\cover.jpg`)
	pub filename: String,
	/// An optional description of the image, max 250 characters
	pub description: Option<String>,
	/// The timestamp at which the image should be displayed
	pub timestamp: Option<Timestamp>,
}

/// Lyrics3v2, as the name suggests, is primarily used to store lyrics. Like its predecessor
/// [`Lyrics3Tag`](crate::id3::lyrics3::v1::Lyrics3Tag), it's meant to act as an extension to
/// an [`Id3v1Tag`](crate::id3::v1::Id3v1Tag).
///
/// This tag type exists for preservation. It is highly recommended to convert them to a more supported
/// type (e.g., [`Id3v2Tag`](crate::id3::v2::Id3v2Tag) or [`ApeTag`](crate::ape::ApeTag)) if possible.
/// You'll have a hard time finding any software that supports these tags.
///
/// ## Writing
///
/// Unlike other formats, Lyrics3v2 has a hard requirement that the destination file has an existing
/// ID3v1 tag. Attempting to write a standalone tag will fail.
///
/// ## Conversions
///
/// ### To `Tag`
///
/// All fields, except `images`, can be converted. The following [`ItemKey`]s are used:
///
/// * `lyrics` - [`ItemKey::Lyrics`]
/// * `info` - [`ItemKey::Comment`]
/// * `author` - [`ItemKey::Lyricist`]
/// * `extended_album_name` - [`ItemKey::AlbumTitle`]
/// * `extended_artist_name` - [`ItemKey::TrackArtist`]
/// * `extended_track_title` - [`ItemKey::TrackTitle`]
///
/// ### From `Tag`
///
/// No special notes.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
#[tag(
	description = "A Lyrics3v2 tag",
	tag_type = Lyrics3v2,
	supported_formats(Mpeg),
	conflicts_with(Lyrics3)
)]
pub struct Lyrics3v2Tag {
	/// Flags that apply to the entire tag
	pub flags: Lyrics3v2TagFlags,
	/// The lyrics of the audio
	///
	/// This is *mostly* an arbitrary text field, but should follow these guidelines:
	///
	/// * Newlines should be `\r\n`
	/// * If the lyrics are synchronized, the timestamps should be in the form: `[mm:ss]`
	///
	/// NOTE: If the lyrics are synchronized, be sure to update the [`Lyrics3v2TagFlags`]
	pub lyrics: Option<String>,
	/// Any additional information about the file
	///
	/// NOTE: This may contain multiple lines (typically separated with `\r\n`)
	pub info: Option<String>,
	/// The author of the lyrics (lyricist)
	pub author: Option<String>,
	/// An extension of the album name in the ID3v1 tag
	///
	/// The first 30 characters of this field **should** match those in the ID3v1 tag.
	///
	/// It is not recommended to use this field if the album name is less than 30 characters long.
	/// Use the album in the ID3v1 tag instead.
	pub extended_album_name: Option<String>,
	/// An extension of the artist name in the ID3v1 tag
	///
	/// The first 30 characters of this field **should** match those in the ID3v1 tag.
	///
	/// It is not recommended to use this field if the artist name is less than 30 characters long.
	/// Use the artist in the ID3v1 tag instead.
	pub extended_artist_name: Option<String>,
	/// An extension of the track title in the ID3v1 tag
	///
	/// The first 30 characters of this field **should** match those in the ID3v1 tag.
	///
	/// It is not recommended to use this field if the track title is less than 30 characters long.
	/// Use the title in the ID3v1 tag instead.
	pub extended_track_title: Option<String>,
	/// Links to related images
	pub images: Vec<Image>,
}

impl Lyrics3v2Tag {
	/// Create a new empty `Lyrics3Tag`
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::id3::lyrics3::v2::Lyrics3v2Tag;
	/// use lofty::tag::TagExt;
	///
	/// let lyrics3_tag = Lyrics3v2Tag::new();
	/// assert!(lyrics3_tag.is_empty());
	/// ```
	pub fn new() -> Self {
		Self::default()
	}
}

impl Accessor for Lyrics3v2Tag {
	impl_accessor!(
		info                 => comment,
		extended_artist_name => artist,
		extended_album_name  => album,
		extended_track_title => title,
	);
}

impl TagExt for Lyrics3v2Tag {
	type RefKey<'a> = &'a ItemKey;

	#[inline]
	fn tag_type(&self) -> TagType {
		TagType::Lyrics3v2
	}

	fn len(&self) -> usize {
		usize::from(self.lyrics.is_some())
			+ usize::from(self.info.is_some())
			+ usize::from(self.author.is_some())
			+ usize::from(self.extended_album_name.is_some())
			+ usize::from(self.extended_artist_name.is_some())
			+ usize::from(self.extended_track_title.is_some())
			+ self.images.len()
	}

	fn contains<'a>(&'a self, key: Self::RefKey<'a>) -> bool {
		match key {
			ItemKey::Lyrics => self.lyrics.is_some(),
			ItemKey::Comment => self.info.is_some(),
			ItemKey::Lyricist => self.author.is_some(),
			ItemKey::AlbumTitle => self.extended_album_name.is_some(),
			ItemKey::TrackArtist => self.extended_artist_name.is_some(),
			ItemKey::TrackTitle => self.extended_track_title.is_some(),
			_ => false,
		}
	}

	fn is_empty(&self) -> bool {
		self.lyrics.is_none()
			&& self.info.is_none()
			&& self.author.is_none()
			&& self.extended_album_name.is_none()
			&& self.extended_artist_name.is_none()
			&& self.extended_track_title.is_none()
			&& self.images.is_empty()
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
	) -> std::result::Result<(), TagEncodingError> {
		Into::<Lyrics3v2TagRef<'_>>::into(self)
			.dump_to(writer, write_options)
			.map_err(Into::into)
	}

	fn clear(&mut self) {
		*self = Self::default();
	}
}

impl TagWriteExt for Lyrics3v2Tag {
	fn save_to<F>(
		&self,
		file: VerifiedFile<'_, F>,
		write_options: WriteOptions,
	) -> std::result::Result<(), FileEncodingError>
	where
		F: FileLike,
	{
		Into::<Lyrics3v2TagRef<'_>>::into(self).write_to(file, write_options)
	}
}

/// The remainder of a [`Lyrics3v2Tag`] after calling [`SplitTag::split_tag`]
#[derive(Debug, Clone, Default)]
pub struct SplitTagRemainder(Lyrics3v2Tag);

impl SplitTag for Lyrics3v2Tag {
	type Remainder = SplitTagRemainder;

	fn split_tag(mut self) -> (Self::Remainder, Tag) {
		fn take(opt: &mut Option<String>, key: ItemKey, tag: &mut Tag) {
			let Some(value) = opt.take() else {
				return;
			};

			// Unlike other formats, empty fields are unlikely to have any significance.
			// We'll just drop them
			if value.is_empty() {
				return;
			}

			tag.insert_text(key, value);
		}

		let Lyrics3v2Tag {
			flags: _,
			lyrics,
			info,
			author,
			extended_album_name,
			extended_artist_name,
			extended_track_title,
			images: _,
		} = &mut self;

		let mut tag = Tag::new(TagType::Lyrics3v2);
		take(lyrics, ItemKey::Lyrics, &mut tag);
		take(info, ItemKey::Comment, &mut tag);
		take(author, ItemKey::Lyricist, &mut tag);
		take(extended_album_name, ItemKey::AlbumTitle, &mut tag);
		take(extended_artist_name, ItemKey::TrackArtist, &mut tag);
		take(extended_track_title, ItemKey::TrackTitle, &mut tag);

		(SplitTagRemainder(self), tag)
	}
}

impl MergeTag for SplitTagRemainder {
	type Merged = Lyrics3v2Tag;

	fn merge_tag(self, mut tag: Tag) -> Self::Merged {
		let SplitTagRemainder(mut merged) = self;
		merged.lyrics = tag.take_strings(ItemKey::Lyrics).next().or(merged.lyrics);
		merged.info = tag.take_strings(ItemKey::Comment).next().or(merged.info);
		merged.author = tag.take_strings(ItemKey::Lyricist).next().or(merged.author);
		merged.extended_album_name = tag
			.take_strings(ItemKey::AlbumTitle)
			.next()
			.or(merged.extended_album_name);
		merged.extended_artist_name = tag
			.take_strings(ItemKey::TrackArtist)
			.next()
			.or(merged.extended_artist_name);
		merged.extended_track_title = tag
			.take_strings(ItemKey::TrackTitle)
			.next()
			.or(merged.extended_track_title);

		merged
	}
}

impl From<Lyrics3v2Tag> for Tag {
	fn from(input: Lyrics3v2Tag) -> Self {
		let (remainder, mut tag) = input.split_tag();

		if unsafe { global_options().preserve_format_specific_items } && !remainder.0.is_empty() {
			tag.companion_tag = Some(CompanionTag::Lyrics3v2(remainder.0));
		}

		tag
	}
}

impl From<Tag> for Lyrics3v2Tag {
	fn from(mut input: Tag) -> Self {
		if unsafe { global_options().preserve_format_specific_items }
			&& let Some(companion) = input.companion_tag.take().and_then(CompanionTag::lyrics3v2)
		{
			return SplitTagRemainder(companion).merge_tag(input);
		}

		SplitTagRemainder::default().merge_tag(input)
	}
}

pub(crate) struct ImageRef<'a> {
	pub filename: &'a str,
	pub description: Option<&'a str>,
	pub timestamp: Option<Timestamp>,
}

impl<'a> From<&'a Image> for ImageRef<'a> {
	fn from(input: &'a Image) -> Self {
		Self {
			filename: &input.filename,
			description: input.description.as_deref(),
			timestamp: input.timestamp,
		}
	}
}

pub(crate) struct Lyrics3v2TagRef<'a> {
	pub flags: Lyrics3v2TagFlags,
	pub lyrics: Option<&'a str>,
	pub info: Option<&'a str>,
	pub author: Option<&'a str>,
	pub extended_album_name: Option<&'a str>,
	pub extended_artist_name: Option<&'a str>,
	pub extended_track_title: Option<&'a str>,
	pub images: Vec<ImageRef<'a>>,
}

impl<'a> Into<Lyrics3v2TagRef<'a>> for &'a Lyrics3v2Tag {
	fn into(self) -> Lyrics3v2TagRef<'a> {
		Lyrics3v2TagRef {
			flags: self.flags,
			lyrics: self.lyrics.as_deref().filter(|s| !s.is_empty()),
			info: self.info.as_deref().filter(|s| !s.is_empty()),
			author: self.author.as_deref().filter(|s| !s.is_empty()),
			extended_album_name: self
				.extended_album_name
				.as_deref()
				.filter(|s| !s.is_empty()),
			extended_artist_name: self
				.extended_artist_name
				.as_deref()
				.filter(|s| !s.is_empty()),
			extended_track_title: self
				.extended_track_title
				.as_deref()
				.filter(|s| !s.is_empty()),
			images: self.images.iter().map(Into::into).collect(),
		}
	}
}

impl<'a> Into<Lyrics3v2TagRef<'a>> for &'a Tag {
	fn into(self) -> Lyrics3v2TagRef<'a> {
		let companion = self
			.companion_tag
			.as_ref()
			.and_then(CompanionTag::lyrics3v2_ref);

		Lyrics3v2TagRef {
			flags: companion.map(|tag| tag.flags).unwrap_or_default(),
			lyrics: self.get_string(ItemKey::Lyrics).filter(|s| !s.is_empty()),
			info: self.get_string(ItemKey::Comment).filter(|s| !s.is_empty()),
			author: self.get_string(ItemKey::Lyricist).filter(|s| !s.is_empty()),
			extended_album_name: self
				.get_string(ItemKey::AlbumTitle)
				.filter(|s| !s.is_empty()),
			extended_artist_name: self
				.get_string(ItemKey::TrackArtist)
				.filter(|s| !s.is_empty()),
			extended_track_title: self
				.get_string(ItemKey::TrackTitle)
				.filter(|s| !s.is_empty()),
			images: companion
				.map(|tag| tag.images.iter().map(Into::into).collect())
				.unwrap_or_default(),
		}
	}
}

impl Lyrics3v2TagRef<'_> {
	pub(crate) fn is_empty(&self) -> bool {
		self.lyrics.is_none()
			&& self.info.is_none()
			&& self.author.is_none()
			&& self.extended_album_name.is_none()
			&& self.extended_artist_name.is_none()
			&& self.extended_track_title.is_none()
			&& self.images.is_empty()
	}

	/// Get the byte length of the encoded tag
	pub(crate) fn len(&self) -> usize {
		// ID (3)
		// Size (5)
		const FIELD_HEADER_LEN: usize = 3 + 5;

		fn field_len(opt: Option<&str>, max_len: usize) -> usize {
			match opt {
				Some(v) => FIELD_HEADER_LEN + v.chars().count().min(max_len),
				None => 0,
			}
		}

		// Implicit indications field
		let mut len = (FIELD_HEADER_LEN + 3)
			+ field_len(self.lyrics, constants::FIELD_MAX_SIZE as usize)
			+ field_len(self.info, constants::FIELD_MAX_SIZE as usize)
			+ field_len(self.author, constants::BASIC_TEXT_FIELD_MAX_SIZE as usize)
			+ field_len(
				self.extended_album_name,
				constants::BASIC_TEXT_FIELD_MAX_SIZE as usize,
			) + field_len(
			self.extended_artist_name,
			constants::BASIC_TEXT_FIELD_MAX_SIZE as usize,
		) + field_len(
			self.extended_track_title,
			constants::BASIC_TEXT_FIELD_MAX_SIZE as usize,
		);

		if !self.images.is_empty() {
			len += FIELD_HEADER_LEN;
		}

		// \r\n
		len += 2 * self.images.len().saturating_sub(1);
		for img in &self.images {
			len += img.filename.chars().count().min(constants::PATH_MAX as usize)
				+ 2 // ||
				+ img
					.description
					.map_or(0, |desc| desc.chars().count())
					.min(constants::BASIC_TEXT_FIELD_MAX_SIZE as usize)
				+ 2 // ||
				+ if img.timestamp.is_some() {
					7 // [00:00]
				} else {
					0
				};
		}

		len
	}

	pub(crate) fn write_to<F>(
		&self,
		file: VerifiedFile<'_, F>,
		write_options: WriteOptions,
	) -> Result<(), FileEncodingError>
	where
		F: FileLike,
	{
		write::write_lyrics3v2(file, self, write_options)
	}

	pub(crate) fn dump_to<W: Write>(
		&mut self,
		writer: &mut W,
		write_options: WriteOptions,
	) -> Result<(), Lyrics3v2EncodingError> {
		let temp = write::encode(self, write_options)?;
		writer.write_all(&temp)?;

		Ok(())
	}
}
