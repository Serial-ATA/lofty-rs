use crate::config::WriteOptions;
use crate::error::{FileEncodingError, TagEncodingError, UnsupportedTagError};
use crate::io::{FileLike, Length, Truncate, VerifiedFile};
use crate::tag::{Accessor, Tag, TagType};

use std::path::Path;

/// A set of common methods between tags
///
/// This provides a set of methods to make interaction with all tags a similar
/// experience.
#[expect(private_bounds, reason = "`TagWriteExt` is internal")]
pub trait TagExt: Accessor + TagWriteExt + Into<Tag> + Sized + private::Sealed {
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
	/// assert!(tag.contains(ItemKey::TrackArtist));
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
	) -> std::result::Result<(), FileEncodingError> {
		TagExt::save_to(
			self,
			&mut std::fs::OpenOptions::new()
				.read(true)
				.write(true)
				.open(path)?,
			write_options,
		)
	}

	/// Save the tag to a [`FileLike`]
	///
	/// # Errors
	///
	/// * The file format could not be determined
	/// * Attempting to write a tag to a format that does not support it.
	fn save_to<F>(
		&self,
		file: &mut F,
		write_options: WriteOptions,
	) -> std::result::Result<(), FileEncodingError>
	where
		F: FileLike,
		FileEncodingError: From<<F as Truncate>::Error>,
		FileEncodingError: From<<F as Length>::Error>,
	{
		let file = VerifiedFile::new(file)?;

		// Empty tag writes are allowed for read-only formats, since they'll be stripped
		if !file.format().tag_support(self.tag_type()).is_writable() && !self.is_empty() {
			return Err(FileEncodingError::from(UnsupportedTagError).with_format(file.format()));
		}

		let format = file.format();
		<Self as TagWriteExt>::save_to(self, file, write_options).map_err(|e| e.with_format(format))
	}

	#[allow(clippy::missing_errors_doc)]
	/// Dump the tag to a writer
	///
	/// This will only write the tag, it will not produce a usable file.
	fn dump_to<W: std::io::Write>(
		&self,
		writer: &mut W,
		write_options: WriteOptions,
	) -> std::result::Result<(), TagEncodingError>;

	/// Remove a tag from a [`Path`]
	///
	/// # Errors
	///
	/// See [`TagExt::remove_from`]
	fn remove_from_path<P: AsRef<Path>>(
		&self,
		path: P,
		write_options: WriteOptions,
	) -> std::result::Result<(), FileEncodingError> {
		self.tag_type().remove_from_path(path, write_options)
	}

	/// Remove a tag from a [`FileLike`]
	///
	/// # Errors
	///
	/// * It is unable to guess the file format
	/// * The format doesn't support the tag
	/// * It is unable to write to the file
	fn remove_from<F>(
		&self,
		file: &mut F,
		write_options: WriteOptions,
	) -> std::result::Result<(), FileEncodingError>
	where
		F: FileLike,
		FileEncodingError: From<<F as Truncate>::Error>,
		FileEncodingError: From<<F as Length>::Error>,
	{
		self.tag_type().remove_from(file, write_options)
	}

	/// Clear the tag, removing all items
	///
	/// NOTE: This will **not** remove any format-specific extras, such as flags
	fn clear(&mut self);
}

pub(crate) trait TagWriteExt {
	fn save_to<F>(
		&self,
		file: VerifiedFile<'_, F>,
		write_options: WriteOptions,
	) -> std::result::Result<(), FileEncodingError>
	where
		F: FileLike,
		FileEncodingError: From<<F as Truncate>::Error>,
		FileEncodingError: From<<F as Length>::Error>;
}

// https://rust-lang.github.io/api-guidelines/future-proofing.html#c-sealed
mod private {
	use crate::ape::ApeTag;
	use crate::id3::v1::Id3v1Tag;
	use crate::id3::v2::Id3v2Tag;
	use crate::iff::aiff::AiffTextChunks;
	use crate::iff::wav::RiffInfoList;
	use crate::mp4::Ilst;
	use crate::ogg::tag::VorbisComments;
	use crate::tag::Tag;

	pub trait Sealed {}

	impl Sealed for AiffTextChunks {}
	impl Sealed for ApeTag {}
	impl Sealed for Id3v1Tag {}
	impl Sealed for Id3v2Tag {}
	impl Sealed for Ilst {}
	impl Sealed for RiffInfoList {}
	impl Sealed for Tag {}
	impl Sealed for VorbisComments {}
}
