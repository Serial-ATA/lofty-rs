use crate::config::WriteOptions;
use crate::error::LoftyError;
use crate::io::{FileLike, Length, Truncate};
use crate::tag::{Accessor, Tag, TagType};

use std::path::Path;

/// A set of common methods between tags
///
/// This provides a set of methods to make interaction with all tags a similar
/// experience.
///
/// This can be implemented downstream to provide a familiar interface for custom tags.
pub trait TagExt: Accessor + Into<Tag> + Sized {
	/// The associated error which can be returned from IO operations
	type Err: From<std::io::Error> + From<LoftyError>;
	/// The type of key used in the tag for non-mutating functions
	type RefKey<'a>
	where
		Self: 'a;

	#[doc(hidden)]
	fn tag_type(&self) -> TagType;

	/// Returns the number of items in the tag
	///
	/// This will also include any extras, such as pictures.
	///
	/// # Example
	///
	/// ```rust
	/// use lofty::tag::{Accessor, ItemKey, Tag, TagExt};
	/// # let tag_type = lofty::tag::TagType::Id3v2;
	///
	/// let mut tag = Tag::new(tag_type);
	/// assert_eq!(tag.len(), 0);
	///
	/// tag.set_artist(String::from("Foo artist"));
	/// assert_eq!(tag.len(), 1);
	/// ```
	fn len(&self) -> usize;

	/// Whether the tag contains an item with the key
	///
	/// # Example
	///
	/// ```rust
	/// use lofty::tag::{Accessor, ItemKey, Tag, TagExt};
	/// # let tag_type = lofty::tag::TagType::Id3v2;
	///
	/// let mut tag = Tag::new(tag_type);
	/// assert!(tag.is_empty());
	///
	/// tag.set_artist(String::from("Foo artist"));
	/// assert!(tag.contains(&ItemKey::TrackArtist));
	/// ```
	fn contains<'a>(&'a self, key: Self::RefKey<'a>) -> bool;

	/// Whether the tag has any items
	///
	/// # Example
	///
	/// ```rust
	/// use lofty::tag::{Accessor, Tag, TagExt};
	/// # let tag_type = lofty::tag::TagType::Id3v2;
	///
	/// let mut tag = Tag::new(tag_type);
	/// assert!(tag.is_empty());
	///
	/// tag.set_artist(String::from("Foo artist"));
	/// assert!(!tag.is_empty());
	/// ```
	fn is_empty(&self) -> bool;

	/// Save the tag to a path
	///
	/// # Errors
	///
	/// * Path doesn't exist
	/// * Path is not writable
	/// * See [`TagExt::save_to`]
	fn save_to_path<P: AsRef<Path>>(
		&self,
		path: P,
		write_options: WriteOptions,
	) -> std::result::Result<(), Self::Err> {
		self.save_to(
			&mut std::fs::OpenOptions::new()
				.read(true)
				.write(true)
				.open(path)?,
			write_options,
		)
	}

	/// Save the tag to a [`File`]
	///
	/// # Errors
	///
	/// * The file format could not be determined
	/// * Attempting to write a tag to a format that does not support it.
	fn save_to<F>(
		&self,
		file: &mut F,
		write_options: WriteOptions,
	) -> std::result::Result<(), Self::Err>
	where
		F: FileLike,
		LoftyError: From<<F as Truncate>::Error>,
		LoftyError: From<<F as Length>::Error>;

	#[allow(clippy::missing_errors_doc)]
	/// Dump the tag to a writer
	///
	/// This will only write the tag, it will not produce a usable file.
	fn dump_to<W: std::io::Write>(
		&self,
		writer: &mut W,
		write_options: WriteOptions,
	) -> std::result::Result<(), Self::Err>;

	/// Remove a tag from a [`Path`]
	///
	/// # Errors
	///
	/// See [`TagExt::remove_from`]
	fn remove_from_path<P: AsRef<Path>>(&self, path: P) -> std::result::Result<(), Self::Err> {
		self.tag_type().remove_from_path(path).map_err(Into::into)
	}

	/// Remove a tag from a [`File`]
	///
	/// # Errors
	///
	/// * It is unable to guess the file format
	/// * The format doesn't support the tag
	/// * It is unable to write to the file
	fn remove_from<F>(&self, file: &mut F) -> std::result::Result<(), Self::Err>
	where
		F: FileLike,
		LoftyError: From<<F as Truncate>::Error>,
		LoftyError: From<<F as Length>::Error>,
	{
		self.tag_type().remove_from(file).map_err(Into::into)
	}

	/// Clear the tag, removing all items
	///
	/// NOTE: This will **not** remove any format-specific extras, such as flags
	fn clear(&mut self);
}