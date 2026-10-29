use super::tagged_file::TaggedFile;
use crate::config::{ParseOptions, WriteOptions};
use crate::error::{FileEncodingError, FileParseError};
use crate::tag::TagType;
use crate::util::io::FileLike;

use std::fs::OpenOptions;
use std::io::{Read, Seek};
use std::path::Path;

/// Provides various methods for interaction with a file
pub trait AudioFile: Into<TaggedFile> {
	/// The struct the file uses for audio properties
	///
	/// Not all formats can use [`FileProperties`](crate::properties::FileProperties) since they may contain additional information
	type Properties;

	/// Read a file from a reader
	///
	/// # Errors
	///
	/// Errors depend on the file and tags being read. See [`FileParseError`](crate::error::FileParseError)
	fn read_from<R>(
		reader: &mut R,
		parse_options: ParseOptions,
	) -> std::result::Result<Self, FileParseError>
	where
		R: Read + Seek,
		Self: Sized;

	/// Attempts to write all tags to a path
	///
	/// # Errors
	///
	/// * `path` does not exist
	/// * `path` is not writable
	/// * See [`AudioFile::save_to`]
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use lofty::config::WriteOptions;
	/// use lofty::file::{AudioFile, TaggedFileExt};
	///
	/// # fn main() -> Result<(), lofty::error::FileEncodingError> {
	/// # let path = "tests/files/assets/minimal/full_test.mp3";
	/// let mut tagged_file = lofty::read_from_path(path)?;
	///
	/// // Edit the tags
	///
	/// tagged_file.save_to_path(path, WriteOptions::default())?;
	/// # Ok(()) }
	/// ```
	fn save_to_path(
		&self,
		path: impl AsRef<Path>,
		write_options: WriteOptions,
	) -> Result<(), FileEncodingError> {
		self.save_to(
			&mut OpenOptions::new().read(true).write(true).open(path)?,
			write_options,
		)
	}

	/// Attempts to write all tags to a file
	///
	/// # Errors
	///
	/// See [`TagExt::save_to`](crate::tag::TagExt::save_to), however this is applicable to every tag in the file.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use lofty::config::WriteOptions;
	/// use lofty::file::{AudioFile, TaggedFileExt};
	/// use std::fs::OpenOptions;
	///
	/// # fn main() -> Result<(), lofty::error::FileEncodingError> {
	/// # let path = "tests/files/assets/minimal/full_test.mp3";
	/// let mut tagged_file = lofty::read_from_path(path)?;
	///
	/// // Edit the tags
	///
	/// let mut file = OpenOptions::new().read(true).write(true).open(path)?;
	/// tagged_file.save_to(&mut file, WriteOptions::default())?;
	/// # Ok(()) }
	/// ```
	fn save_to<F>(
		&self,
		file: &mut F,
		write_options: WriteOptions,
	) -> Result<(), FileEncodingError>
	where
		F: FileLike;

	/// Returns a reference to the file's properties
	fn properties(&self) -> &Self::Properties;
	/// Returns an iterator over the [`TagType`]s of the tags contained in the file
	///
	/// NOTE: The order of the tag types is not guaranteed to be stable
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::file::{AudioFile, TaggedFileExt};
	/// use lofty::tag::{Tag, TagType};
	///
	/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// # let mp3_path = "tests/files/assets/minimal/full_test.mp3";
	/// let mut tagged_file = lofty::read_from_path(mp3_path)?;
	/// # tagged_file.clear();
	///
	/// // Starting off with an untagged file
	/// assert!(tagged_file.tag_types().next().is_none());
	///
	/// tagged_file.insert_tag(Tag::new(TagType::Id3v2));
	/// tagged_file.insert_tag(Tag::new(TagType::Id3v1));
	///
	/// let tag_types: Vec<_> = tagged_file.tag_types().collect();
	/// assert_eq!(tag_types, vec![TagType::Id3v2, TagType::Id3v1]);
	/// # Ok(()) }
	/// ```
	fn tag_types(&self) -> impl Iterator<Item = TagType> + Clone;
	/// Find the conflicting tags present in the file, if any
	///
	/// NOTE: Conflicts can only arise from user edits. Lofty will never produce a file with conflicts
	///       from parsing.
	///
	/// Returns an iterator of the conflicting [`TagType`]s.
	///
	/// See [`TagType::conflicts_with()`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::file::{AudioFile, TaggedFileExt};
	/// use lofty::tag::{Tag, TagType};
	///
	/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// # let mp3_path = "tests/files/assets/minimal/full_test.mp3";
	/// let mut tagged_file = lofty::read_from_path(mp3_path)?;
	///
	/// // The parser never produces a file with conflicts
	/// assert!(tagged_file.conflicts().next().is_none());
	///
	/// // MP3 supports Lyrics3 and Lyrics3v2, but only one can exist at a time
	/// tagged_file.insert_tag(Tag::new(TagType::Lyrics3));
	/// tagged_file.insert_tag(Tag::new(TagType::Lyrics3v2));
	///
	/// assert_eq!(
	/// 	tagged_file.conflicts().next(),
	/// 	Some((TagType::Lyrics3, TagType::Lyrics3v2))
	/// );
	/// # Ok(()) }
	/// ```
	fn conflicts(&self) -> impl Iterator<Item = (TagType, TagType)> {
		let mut outer = self.tag_types();
		let mut current = outer.next().map(|t| (t, outer.clone()));

		core::iter::from_fn(move || {
			while let Some((tag_a, ref mut inner)) = current {
				for tag_b in inner.by_ref() {
					if tag_a.conflicts_with(tag_b) {
						return Some((tag_a, tag_b));
					}
				}
				current = outer.next().map(|t| (t, outer.clone()));
			}
			None
		})
	}
}
