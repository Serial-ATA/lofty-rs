use super::tagged_file::TaggedFile;
use crate::config::{ParseOptions, WriteOptions};
use crate::error::Result;
use crate::tag::TagType;

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek};
use std::path::Path;

/// Provides various methods for interaction with a file
pub trait AudioFile: Into<TaggedFile> {
	/// The struct the file uses for audio properties
	///
	/// Not all formats can use [`FileProperties`] since they may contain additional information
	type Properties;

	/// Read a file from a reader
	///
	/// # Errors
	///
	/// Errors depend on the file and tags being read. See [`LoftyError`](crate::LoftyError)
	fn read_from<R>(reader: &mut R, parse_options: ParseOptions) -> Result<Self>
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
	/// # fn main() -> lofty::error::Result<()> {
	/// # let path = "tests/files/assets/minimal/full_test.mp3";
	/// let mut tagged_file = lofty::read_from_path(path)?;
	///
	/// // Edit the tags
	///
	/// tagged_file.save_to_path(path, WriteOptions::default())?;
	/// # Ok(()) }
	/// ```
	fn save_to_path(&self, path: impl AsRef<Path>, write_options: WriteOptions) -> Result<()> {
		self.save_to(
			&mut OpenOptions::new().read(true).write(true).open(path)?,
			write_options,
		)
	}

	/// Attempts to write all tags to a file
	///
	/// # Errors
	///
	/// See [`Tag::save_to`], however this is applicable to every tag in the file.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use lofty::config::WriteOptions;
	/// use lofty::file::{AudioFile, TaggedFileExt};
	/// use std::fs::OpenOptions;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// # let path = "tests/files/assets/minimal/full_test.mp3";
	/// let mut tagged_file = lofty::read_from_path(path)?;
	///
	/// // Edit the tags
	///
	/// let mut file = OpenOptions::new().read(true).write(true).open(path)?;
	/// tagged_file.save_to(&mut file, WriteOptions::default())?;
	/// # Ok(()) }
	/// ```
	fn save_to(&self, file: &mut File, write_options: WriteOptions) -> Result<()>;

	/// Returns a reference to the file's properties
	fn properties(&self) -> &Self::Properties;
	/// Checks if the file contains any tags
	fn contains_tag(&self) -> bool;
	/// Checks if the file contains the given [`TagType`]
	fn contains_tag_type(&self, tag_type: TagType) -> bool;
}
