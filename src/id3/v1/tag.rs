use crate::error::{LoftyError, Result};
use crate::id3::v1::constants::GENRES;
use crate::tag::item::{ItemKey, ItemValue, TagItem};
use crate::tag::{Tag, TagType};
use crate::traits::{Accessor, TagExt};

use std::borrow::Cow;
use std::fs::{File, OpenOptions};
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
/// ### From `Tag`
///
/// Two checks are performed when converting a genre:
///
/// * [`GENRES`] contains the string
/// * The [`ItemValue`](crate::ItemValue) can be parsed into a `u8`
#[derive(Default, Debug, PartialEq, Eq, Clone)]
#[tag(
	description = "An ID3v1 tag",
	supported_formats(AAC, APE, MPEG, WavPack)
)]
pub struct ID3v1Tag {
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

impl Accessor for ID3v1Tag {
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

	fn year(&self) -> Option<u32> {
		if let Some(ref year) = self.year {
			if let Ok(y) = year.parse() {
				return Some(y);
			}
		}

		None
	}

	fn set_year(&mut self, value: u32) {
		self.year = Some(value.to_string());
	}

	fn remove_year(&mut self) {
		self.year = None;
	}
}

impl TagExt for ID3v1Tag {
	type Err = LoftyError;
	type RefKey<'a> = &'a ItemKey;

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

	fn save_to_path<P: AsRef<Path>>(&self, path: P) -> std::result::Result<(), Self::Err> {
		self.save_to(&mut OpenOptions::new().read(true).write(true).open(path)?)
	}

	fn save_to(&self, file: &mut File) -> std::result::Result<(), Self::Err> {
		Into::<Id3v1TagRef<'_>>::into(self).write_to(file)
	}

	/// Dumps the tag to a writer
	///
	/// # Errors
	///
	/// * [`std::io::Error`]
	fn dump_to<W: Write>(&self, writer: &mut W) -> std::result::Result<(), Self::Err> {
		Into::<Id3v1TagRef<'_>>::into(self).dump_to(writer)
	}

	fn remove_from_path<P: AsRef<Path>>(&self, path: P) -> std::result::Result<(), Self::Err> {
		TagType::ID3v1.remove_from_path(path)
	}

	fn remove_from(&self, file: &mut File) -> std::result::Result<(), Self::Err> {
		TagType::ID3v1.remove_from(file)
	}

	fn clear(&mut self) {
		*self = Self::default();
	}
}

impl From<ID3v1Tag> for Tag {
	fn from(input: ID3v1Tag) -> Self {
		let mut tag = Self::new(TagType::ID3v1);

		input.title.map(|t| tag.insert_text(ItemKey::TrackTitle, t));
		input
			.artist
			.map(|a| tag.insert_text(ItemKey::TrackArtist, a));
		input.album.map(|a| tag.insert_text(ItemKey::AlbumTitle, a));
		input.year.map(|y| tag.insert_text(ItemKey::Year, y));
		input.comment.map(|c| tag.insert_text(ItemKey::Comment, c));

		if let Some(t) = input.track_number {
			tag.items.push(TagItem::new(
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

impl From<Tag> for ID3v1Tag {
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

pub(crate) struct Id3v1TagRef<'a> {
	pub title: Option<&'a str>,
	pub artist: Option<&'a str>,
	pub album: Option<&'a str>,
	pub year: Option<&'a str>,
	pub comment: Option<&'a str>,
	pub track_number: Option<u8>,
	pub genre: Option<u8>,
}

impl<'a> Into<Id3v1TagRef<'a>> for &'a ID3v1Tag {
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
	pub(super) fn is_empty(&self) -> bool {
		self.title.is_none()
			&& self.artist.is_none()
			&& self.album.is_none()
			&& self.year.is_none()
			&& self.comment.is_none()
			&& self.track_number.is_none()
			&& self.genre.is_none()
	}

	pub(crate) fn write_to(&self, file: &mut File) -> Result<()> {
		super::write::write_id3v1(file, self)
	}

	pub(crate) fn dump_to<W: Write>(&mut self, writer: &mut W) -> Result<()> {
		let temp = super::write::encode(self)?;
		writer.write_all(&temp)?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use crate::id3::v1::ID3v1Tag;
	use crate::{Tag, TagExt, TagType};

	#[test]
	fn parse_id3v1() {
		let expected_tag = ID3v1Tag {
			title: Some(String::from("Foo title")),
			artist: Some(String::from("Bar artist")),
			album: Some(String::from("Baz album")),
			year: Some(String::from("1984")),
			comment: Some(String::from("Qux comment")),
			track_number: Some(1),
			genre: Some(32),
		};

		let tag = crate::tag::utils::test_utils::read_path("tests/tags/assets/test.id3v1");
		let parsed_tag = crate::id3::v1::read::parse_id3v1(tag.try_into().unwrap());

		assert_eq!(expected_tag, parsed_tag);
	}

	#[test]
	fn id3v2_re_read() {
		let tag = crate::tag::utils::test_utils::read_path("tests/tags/assets/test.id3v1");
		let parsed_tag = crate::id3::v1::read::parse_id3v1(tag.try_into().unwrap());

		let mut writer = Vec::new();
		parsed_tag.dump_to(&mut writer).unwrap();

		let temp_parsed_tag = crate::id3::v1::read::parse_id3v1(writer.try_into().unwrap());

		assert_eq!(parsed_tag, temp_parsed_tag);
	}

	#[test]
	fn id3v1_to_tag() {
		let tag_bytes = crate::tag::utils::test_utils::read_path("tests/tags/assets/test.id3v1");
		let id3v1 = crate::id3::v1::read::parse_id3v1(tag_bytes.try_into().unwrap());

		let tag: Tag = id3v1.into();

		crate::tag::utils::test_utils::verify_tag(&tag, true, true);
	}

	#[test]
	fn tag_to_id3v1() {
		let tag = crate::tag::utils::test_utils::create_tag(TagType::ID3v1);

		let id3v1_tag: ID3v1Tag = tag.into();

		assert_eq!(id3v1_tag.title.as_deref(), Some("Foo title"));
		assert_eq!(id3v1_tag.artist.as_deref(), Some("Bar artist"));
		assert_eq!(id3v1_tag.album.as_deref(), Some("Baz album"));
		assert_eq!(id3v1_tag.comment.as_deref(), Some("Qux comment"));
		assert_eq!(id3v1_tag.track_number, Some(1));
		assert_eq!(id3v1_tag.genre, Some(32));
	}
}
