//! Lyrics3 v1 items
//!
//! # Lyrics3 notes
//!
//! See also: [`Lyrics3Tag`]

pub(crate) mod constants;
pub use constants::MAX_LYRICS_LEN;
pub(crate) mod errors;
mod read;
#[cfg(test)]
mod tests;
mod write;

use crate::config::WriteOptions;
use crate::error::{FileEncodingError, TagEncodingError};
use crate::id3::lyrics3::v1::errors::Lyrics3EncodingError;
use crate::io::{FileLike, VerifiedFile};
use crate::prelude::ItemKey;
use crate::tag::{Accessor, MergeTag, SplitTag, Tag, TagExt, TagType, TagWriteExt};

use std::io::Write;

use lofty_attr::tag;

/// Lyrics3 tags serve a single purpose: storing lyrics. They're meant to act as an extension to
/// an [`Id3v1Tag`](crate::id3::v1::Id3v1Tag).
///
/// This tag type exists for preservation. It is highly recommended to convert them to a more supported
/// type (e.g., [`Id3v2Tag`](crate::id3::v2::Id3v2Tag) or [`ApeTag`](crate::ape::ApeTag)) if possible.
/// You'll have a hard time finding any software that supports these tags.
///
/// ## Writing
///
/// Unlike other formats, Lyrics3 has a hard requirement that the destination file has an existing
/// ID3v1 tag. Attempting to write a standalone tag will fail.
///
/// ## Conversions
///
/// ### To `Tag`
///
/// This will always use [`ItemKey::Lyrics`], as the lyrics may be synchronized.
///
/// ### From `Tag`
///
/// No special notes.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
#[tag(
	description = "A Lyrics3 tag",
	tag_type = Lyrics3,
	supported_formats(Mpeg),
	conflicts_with(Lyrics3v2)
)]
pub struct Lyrics3Tag {
	/// The lyrics of the audio
	///
	/// This is *mostly* an arbitrary text field, but should follow these guidelines:
	///
	/// * Newlines should be `\r\n`
	/// * If the lyrics are synchronized, the timestamps should be in the form: `[mm:ss]`
	///
	/// Additionally, it has the following hard constraints:
	///
	/// * Max length of [`MAX_LYRICS_LEN`] bytes
	/// 	* When writing, the text will be truncated if it is too long
	/// * Restricted to the [`TextEncoding::Latin1`](crate::TextEncoding::Latin1) character set
	/// 	* But the character ÿ (`0xFF`) is forbidden. If it is encountered, see [`WriteOptions::lossy_text_encoding()`]
	///       for the behavior.
	pub lyrics: String,
}

impl Lyrics3Tag {
	/// Create a new empty `Lyrics3Tag`
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::id3::lyrics3::v1::Lyrics3Tag;
	/// use lofty::tag::TagExt;
	///
	/// let lyrics3_tag = Lyrics3Tag::new();
	/// assert!(lyrics3_tag.is_empty());
	/// ```
	pub fn new() -> Self {
		Self::default()
	}
}

impl Accessor for Lyrics3Tag {}

impl TagExt for Lyrics3Tag {
	type RefKey<'a> = &'a ItemKey;

	#[inline]
	fn tag_type(&self) -> TagType {
		TagType::Lyrics3
	}

	fn len(&self) -> usize {
		usize::from(!self.lyrics.is_empty())
	}

	fn contains<'a>(&'a self, key: Self::RefKey<'a>) -> bool {
		match key {
			ItemKey::Lyrics => !self.lyrics.is_empty(),
			_ => false,
		}
	}

	fn is_empty(&self) -> bool {
		self.lyrics.is_empty()
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
		Into::<Lyrics3TagRef<'_>>::into(self)
			.dump_to(writer, write_options)
			.map_err(Into::into)
	}

	fn clear(&mut self) {
		*self = Self::default();
	}
}

impl TagWriteExt for Lyrics3Tag {
	fn save_to<F>(
		&self,
		file: VerifiedFile<'_, F>,
		write_options: WriteOptions,
	) -> std::result::Result<(), FileEncodingError>
	where
		F: FileLike,
	{
		Into::<Lyrics3TagRef<'_>>::into(self).write_to(file, write_options)
	}
}

/// The remainder of a [`Lyrics3Tag`] after calling [`SplitTag::split_tag`]
#[derive(Debug, Clone, Default)]
pub struct SplitTagRemainder;

impl SplitTag for Lyrics3Tag {
	type Remainder = SplitTagRemainder;

	fn split_tag(mut self) -> (Self::Remainder, Tag) {
		let mut tag = Tag::new(TagType::Lyrics3);
		if !self.lyrics.is_empty() {
			tag.insert_text(ItemKey::Lyrics, std::mem::take(&mut self.lyrics));
		}

		(SplitTagRemainder, tag)
	}
}

impl MergeTag for SplitTagRemainder {
	type Merged = Lyrics3Tag;

	fn merge_tag(self, tag: Tag) -> Self::Merged {
		tag.into()
	}
}

impl From<Lyrics3Tag> for Tag {
	fn from(input: Lyrics3Tag) -> Self {
		input.split_tag().1
	}
}

impl From<Tag> for Lyrics3Tag {
	fn from(mut input: Tag) -> Self {
		let lyrics = input.take_strings(ItemKey::Lyrics).next();
		Self {
			lyrics: lyrics.unwrap_or_default(),
		}
	}
}

pub(crate) struct Lyrics3TagRef<'a> {
	pub lyrics: Option<&'a str>,
}

impl<'a> Into<Lyrics3TagRef<'a>> for &'a Lyrics3Tag {
	fn into(self) -> Lyrics3TagRef<'a> {
		Lyrics3TagRef {
			lyrics: if self.lyrics.is_empty() {
				None
			} else {
				Some(&self.lyrics)
			},
		}
	}
}

impl<'a> Into<Lyrics3TagRef<'a>> for &'a Tag {
	fn into(self) -> Lyrics3TagRef<'a> {
		Lyrics3TagRef {
			lyrics: self.get_string(ItemKey::Lyrics).filter(|s| !s.is_empty()),
		}
	}
}

impl Lyrics3TagRef<'_> {
	pub(crate) fn write_to<F>(
		&self,
		file: VerifiedFile<'_, F>,
		write_options: WriteOptions,
	) -> Result<(), FileEncodingError>
	where
		F: FileLike,
	{
		write::write_lyrics3(file, self, write_options)
	}

	pub(crate) fn dump_to<W: Write>(
		&mut self,
		writer: &mut W,
		write_options: WriteOptions,
	) -> Result<(), Lyrics3EncodingError> {
		let temp = write::encode(self, write_options)?;
		writer.write_all(&temp)?;

		Ok(())
	}
}
