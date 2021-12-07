use crate::error::Result;
use crate::logic::id3::v1::constants::GENRES;
use crate::types::item::{ItemKey, ItemValue, TagItem};
use crate::types::tag::{Tag, TagType};

use std::fs::File;

#[derive(Default, Debug, PartialEq)]
/// An ID3v1 tag
///
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
/// ### From `Tag`
///
/// Two checks are performed when converting a genre:
///
/// * [`GENRES`] contains the string
/// * The [`ItemValue`](crate::ItemValue) can be parsed into a `u8`
pub struct Id3v1Tag {
	/// Track title, 30 bytes max
	pub title: Option<String>,
	/// Track artist, 30 bytes max
	pub artist: Option<String>,
	/// Album title, 30 bytes max
	pub album: Option<String>,
	/// Release year, 4 bytes max
	pub year: Option<String>,
	/// A short comment
	///
	/// The number of bytes differs between versions, but not much.
	/// A V1 tag may have been read, which limits this field to 30 bytes.
	/// A V1.1 tag, however, only has 28 bytes available.
	///
	/// Lofty will *always* write a V1.1 tag.
	pub comment: Option<String>,
	/// The track number, 1 byte max
	///
	/// Issues:
	///
	/// * The track number **cannot** be 0. Many readers, including Lofty,
	/// look for a zeroed byte at the end of the comment to differentiate
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
	/// Returns `true` if the tag contains no data
	pub fn is_empty(&self) -> bool {
		self.title.is_none()
			&& self.artist.is_none()
			&& self.album.is_none()
			&& self.year.is_none()
			&& self.comment.is_none()
			&& self.track_number.is_none()
			&& self.genre.is_none()
	}

	#[allow(clippy::missing_errors_doc)]
	/// Parses an [`Id3v1Tag`] from an array
	///
	/// NOTE: This is **NOT** for reading from a file. This is used internally.
	pub fn read_from(tag: [u8; 128]) -> Self {
		super::read::parse_id3v1(tag)
	}

	/// Write the tag to a file
	///
	/// # Errors
	///
	/// * Attempting to write the tag to a format that does not support it
	pub fn write_to(&self, file: &mut File) -> Result<()> {
		Into::<Id3v1TagRef>::into(self).write_to(file)
	}
}

impl From<Id3v1Tag> for Tag {
	fn from(input: Id3v1Tag) -> Self {
		let mut tag = Self::new(TagType::Id3v1);

		input.title.map(|t| tag.insert_text(ItemKey::TrackTitle, t));
		input
			.artist
			.map(|a| tag.insert_text(ItemKey::TrackArtist, a));
		input.album.map(|a| tag.insert_text(ItemKey::AlbumTitle, a));
		input.year.map(|y| tag.insert_text(ItemKey::Year, y));
		input.comment.map(|c| tag.insert_text(ItemKey::Comment, c));

		if let Some(t) = input.track_number {
			tag.insert_item_unchecked(TagItem::new(
				ItemKey::TrackNumber,
				ItemValue::Text(t.to_string()),
			))
		}

		if let Some(genre_index) = input.genre {
			if let Some(genre) = GENRES.get(genre_index as usize) {
				tag.insert_text(ItemKey::Genre, (*genre).to_string());
			}
		}

		tag
	}
}

impl From<Tag> for Id3v1Tag {
	fn from(input: Tag) -> Self {
		Self {
			title: input.get_string(&ItemKey::TrackTitle).map(str::to_owned),
			artist: input.get_string(&ItemKey::TrackArtist).map(str::to_owned),
			album: input.get_string(&ItemKey::AlbumTitle).map(str::to_owned),
			year: input.get_string(&ItemKey::Year).map(str::to_owned),
			comment: input.get_string(&ItemKey::Comment).map(str::to_owned),
			track_number: input
				.get_string(&ItemKey::TrackNumber)
				.map(|g| g.parse::<u8>().ok())
				.and_then(|g| g),
			genre: input
				.get_string(&ItemKey::Genre)
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

pub(in crate::logic) struct Id3v1TagRef<'a> {
	pub title: Option<&'a str>,
	pub artist: Option<&'a str>,
	pub album: Option<&'a str>,
	pub year: Option<&'a str>,
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
			year: self.year.as_deref(),
			comment: self.comment.as_deref(),
			track_number: self.track_number,
			genre: self.genre,
		}
	}
}

impl<'a> Into<Id3v1TagRef<'a>> for &'a Tag {
	fn into(self) -> Id3v1TagRef<'a> {
		Id3v1TagRef {
			title: self.get_string(&ItemKey::TrackTitle),
			artist: self.get_string(&ItemKey::TrackArtist),
			album: self.get_string(&ItemKey::AlbumTitle),
			year: self.get_string(&ItemKey::Year),
			comment: self.get_string(&ItemKey::Comment),
			track_number: self
				.get_string(&ItemKey::TrackNumber)
				.map(|g| g.parse::<u8>().ok())
				.and_then(|g| g),
			genre: self
				.get_string(&ItemKey::Genre)
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

impl<'a> Id3v1TagRef<'a> {
	pub(crate) fn write_to(&self, file: &mut File) -> Result<()> {
		super::write::write_id3v1(file, self)
	}

	pub(super) fn is_empty(&self) -> bool {
		self.title.is_none()
			&& self.artist.is_none()
			&& self.album.is_none()
			&& self.year.is_none()
			&& self.comment.is_none()
			&& self.track_number.is_none()
			&& self.genre.is_none()
	}
}
