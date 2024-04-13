use super::audio_file::AudioFile;
use super::file_type::FileType;
use crate::config::{ParseOptions, WriteOptions};
use crate::error::Result;
use crate::properties::FileProperties;
use crate::tag::{Tag, TagExt, TagType};

use std::fs::File;
use std::io::{Read, Seek};

/// Provides a common interface between [`TaggedFile`] and [`BoundTaggedFile`]
pub trait TaggedFileExt {
	/// Returns the file's [`FileType`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::file::{FileType, TaggedFileExt};
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// # let path_to_mp3 = "tests/files/assets/minimal/full_test.mp3";
	/// let mut tagged_file = lofty::read_from_path(path_to_mp3)?;
	///
	/// assert_eq!(tagged_file.file_type(), FileType::Mpeg);
	/// # Ok(()) }
	/// ```
	fn file_type(&self) -> FileType;

	/// Returns all tags
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::file::{FileType, TaggedFileExt};
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// # let path_to_mp3 = "tests/files/assets/minimal/full_test.mp3";
	/// // An MP3 file with 3 tags
	/// let mut tagged_file = lofty::read_from_path(path_to_mp3)?;
	///
	/// let tags = tagged_file.tags();
	///
	/// assert_eq!(tags.len(), 3);
	/// # Ok(()) }
	/// ```
	fn tags(&self) -> &[Tag];

	/// Returns the file type's primary [`TagType`]
	///
	/// See [`FileType::primary_tag_type`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::file::TaggedFileExt;
	/// use lofty::tag::TagType;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// # let path_to_mp3 = "tests/files/assets/minimal/full_test.mp3";
	/// let mut tagged_file = lofty::read_from_path(path_to_mp3)?;
	///
	/// assert_eq!(tagged_file.primary_tag_type(), TagType::Id3v2);
	/// # Ok(()) }
	/// ```
	fn primary_tag_type(&self) -> TagType {
		self.file_type().primary_tag_type()
	}

	/// Determines whether the file supports the given [`TagType`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::file::TaggedFileExt;
	/// use lofty::tag::TagType;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// # let path_to_mp3 = "tests/files/assets/minimal/full_test.mp3";
	/// let mut tagged_file = lofty::read_from_path(path_to_mp3)?;
	///
	/// assert!(tagged_file.supports_tag_type(TagType::Id3v2));
	/// # Ok(()) }
	/// ```
	fn supports_tag_type(&self, tag_type: TagType) -> bool {
		self.file_type().supports_tag_type(tag_type)
	}

	/// Get a reference to a specific [`TagType`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::file::TaggedFileExt;
	/// use lofty::tag::TagType;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// # let path_to_mp3 = "tests/files/assets/minimal/full_test.mp3";
	/// // Read an MP3 file with an ID3v2 tag
	/// let mut tagged_file = lofty::read_from_path(path_to_mp3)?;
	///
	/// // An ID3v2 tag
	/// let tag = tagged_file.tag(TagType::Id3v2);
	///
	/// assert!(tag.is_some());
	/// assert_eq!(tag.unwrap().tag_type(), TagType::Id3v2);
	/// # Ok(()) }
	/// ```
	fn tag(&self, tag_type: TagType) -> Option<&Tag>;

	/// Get a mutable reference to a specific [`TagType`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::file::TaggedFileExt;
	/// use lofty::tag::TagType;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// # let path_to_mp3 = "tests/files/assets/minimal/full_test.mp3";
	/// // Read an MP3 file with an ID3v2 tag
	/// let mut tagged_file = lofty::read_from_path(path_to_mp3)?;
	///
	/// // An ID3v2 tag
	/// let tag = tagged_file.tag(TagType::Id3v2);
	///
	/// assert!(tag.is_some());
	/// assert_eq!(tag.unwrap().tag_type(), TagType::Id3v2);
	///
	/// // Alter the tag...
	/// # Ok(()) }
	/// ```
	fn tag_mut(&mut self, tag_type: TagType) -> Option<&mut Tag>;

	/// Returns the primary tag
	///
	/// See [`FileType::primary_tag_type`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::file::TaggedFileExt;
	/// use lofty::tag::TagType;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// # let path_to_mp3 = "tests/files/assets/minimal/full_test.mp3";
	/// // Read an MP3 file with an ID3v2 tag
	/// let mut tagged_file = lofty::read_from_path(path_to_mp3)?;
	///
	/// // An ID3v2 tag
	/// let tag = tagged_file.primary_tag();
	///
	/// assert!(tag.is_some());
	/// assert_eq!(tag.unwrap().tag_type(), TagType::Id3v2);
	/// # Ok(()) }
	/// ```
	fn primary_tag(&self) -> Option<&Tag> {
		self.tag(self.primary_tag_type())
	}

	/// Gets a mutable reference to the file's "Primary tag"
	///
	/// See [`FileType::primary_tag_type`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::file::TaggedFileExt;
	/// use lofty::tag::TagType;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// # let path_to_mp3 = "tests/files/assets/minimal/full_test.mp3";
	/// // Read an MP3 file with an ID3v2 tag
	/// let mut tagged_file = lofty::read_from_path(path_to_mp3)?;
	///
	/// // An ID3v2 tag
	/// let tag = tagged_file.primary_tag_mut();
	///
	/// assert!(tag.is_some());
	/// assert_eq!(tag.unwrap().tag_type(), TagType::Id3v2);
	///
	/// // Alter the tag...
	/// # Ok(()) }
	/// ```
	fn primary_tag_mut(&mut self) -> Option<&mut Tag> {
		self.tag_mut(self.primary_tag_type())
	}

	/// Gets the first tag, if there are any
	///
	/// NOTE: This will grab the first available tag, you cannot rely on the result being
	/// a specific type
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::file::TaggedFileExt;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// # let path = "tests/files/assets/minimal/full_test.mp3";
	/// // A file we know has tags
	/// let mut tagged_file = lofty::read_from_path(path)?;
	///
	/// // A tag of a (currently) unknown type
	/// let tag = tagged_file.first_tag();
	/// assert!(tag.is_some());
	/// # Ok(()) }
	/// ```
	fn first_tag(&self) -> Option<&Tag> {
		self.tags().first()
	}

	/// Gets a mutable reference to the first tag, if there are any
	///
	/// NOTE: This will grab the first available tag, you cannot rely on the result being
	/// a specific type
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::file::TaggedFileExt;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// # let path = "tests/files/assets/minimal/full_test.mp3";
	/// // A file we know has tags
	/// let mut tagged_file = lofty::read_from_path(path)?;
	///
	/// // A tag of a (currently) unknown type
	/// let tag = tagged_file.first_tag_mut();
	/// assert!(tag.is_some());
	///
	/// // Alter the tag...
	/// # Ok(()) }
	/// ```
	fn first_tag_mut(&mut self) -> Option<&mut Tag>;

	/// Inserts a [`Tag`]
	///
	/// NOTE: This will do nothing if the [`FileType`] does not support
	/// the [`TagType`]. See [`FileType::supports_tag_type`]
	///
	/// If a tag is replaced, it will be returned
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::file::{AudioFile, TaggedFileExt};
	/// use lofty::tag::{Tag, TagType};
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// # let path_to_mp3 = "tests/files/assets/minimal/full_test.mp3";
	/// // Read an MP3 file without an ID3v2 tag
	/// let mut tagged_file = lofty::read_from_path(path_to_mp3)?;
	/// # let _ = tagged_file.remove(TagType::Id3v2); // sneaky
	///
	/// assert!(!tagged_file.contains_tag_type(TagType::Id3v2));
	///
	/// // Insert the ID3v2 tag
	/// let new_id3v2_tag = Tag::new(TagType::Id3v2);
	/// tagged_file.insert_tag(new_id3v2_tag);
	///
	/// assert!(tagged_file.contains_tag_type(TagType::Id3v2));
	/// # Ok(()) }
	/// ```
	fn insert_tag(&mut self, tag: Tag) -> Option<Tag>;

	/// Removes a specific [`TagType`] and returns it
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::file::{AudioFile, TaggedFileExt};
	/// use lofty::tag::TagType;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// # let path_to_mp3 = "tests/files/assets/minimal/full_test.mp3";
	/// // Read an MP3 file containing an ID3v2 tag
	/// let mut tagged_file = lofty::read_from_path(path_to_mp3)?;
	///
	/// assert!(tagged_file.contains_tag_type(TagType::Id3v2));
	///
	/// // Take the ID3v2 tag
	/// let id3v2 = tagged_file.remove(TagType::Id3v2);
	///
	/// assert!(!tagged_file.contains_tag_type(TagType::Id3v2));
	/// # Ok(()) }
	/// ```
	fn remove(&mut self, tag_type: TagType) -> Option<Tag>;

	/// Removes all tags from the file
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::file::TaggedFileExt;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// # let path = "tests/files/assets/minimal/full_test.mp3";
	/// let mut tagged_file = lofty::read_from_path(path)?;
	///
	/// tagged_file.clear();
	///
	/// assert!(tagged_file.tags().is_empty());
	/// # Ok(()) }
	/// ```
	fn clear(&mut self);
}

/// A generic representation of a file
///
/// This is used when the [`FileType`] has to be guessed
pub struct TaggedFile {
	/// The file's type
	pub(crate) ty: FileType,
	/// The file's audio properties
	pub(crate) properties: FileProperties,
	/// A collection of the file's tags
	pub(crate) tags: Vec<Tag>,
}

impl TaggedFile {
	#[doc(hidden)]
	/// This exists for use in `lofty_attr`, there's no real use for this externally
	#[must_use]
	pub const fn new(ty: FileType, properties: FileProperties, tags: Vec<Tag>) -> Self {
		Self {
			ty,
			properties,
			tags,
		}
	}

	/// Changes the [`FileType`]
	///
	/// NOTES:
	///
	/// * This will remove any tag the format does not support. See [`FileType::supports_tag_type`]
	/// * This will reset the [`FileProperties`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::file::{AudioFile, FileType, TaggedFileExt};
	/// use lofty::tag::TagType;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// # let path_to_mp3 = "tests/files/assets/minimal/full_test.mp3";
	/// // Read an MP3 file containing an ID3v2 tag
	/// let mut tagged_file = lofty::read_from_path(path_to_mp3)?;
	///
	/// assert!(tagged_file.contains_tag_type(TagType::Id3v2));
	///
	/// // Remap our MP3 file to WavPack, which doesn't support ID3v2
	/// tagged_file.change_file_type(FileType::WavPack);
	///
	/// assert!(!tagged_file.contains_tag_type(TagType::Id3v2));
	/// # Ok(()) }
	/// ```
	pub fn change_file_type(&mut self, file_type: FileType) {
		self.ty = file_type;
		self.properties = FileProperties::default();
		self.tags
			.retain(|t| self.ty.supports_tag_type(t.tag_type()));
	}
}

impl TaggedFileExt for TaggedFile {
	fn file_type(&self) -> FileType {
		self.ty
	}

	fn tags(&self) -> &[Tag] {
		self.tags.as_slice()
	}

	fn tag(&self, tag_type: TagType) -> Option<&Tag> {
		self.tags.iter().find(|i| i.tag_type() == tag_type)
	}

	fn tag_mut(&mut self, tag_type: TagType) -> Option<&mut Tag> {
		self.tags.iter_mut().find(|i| i.tag_type() == tag_type)
	}

	fn first_tag_mut(&mut self) -> Option<&mut Tag> {
		self.tags.first_mut()
	}

	fn insert_tag(&mut self, tag: Tag) -> Option<Tag> {
		let tag_type = tag.tag_type();

		if self.supports_tag_type(tag_type) {
			let ret = self.remove(tag_type);
			self.tags.push(tag);

			return ret;
		}

		None
	}

	fn remove(&mut self, tag_type: TagType) -> Option<Tag> {
		self.tags
			.iter()
			.position(|t| t.tag_type() == tag_type)
			.map(|pos| self.tags.remove(pos))
	}

	fn clear(&mut self) {
		self.tags.clear()
	}
}

impl AudioFile for TaggedFile {
	type Properties = FileProperties;

	fn read_from<R>(reader: &mut R, parse_options: ParseOptions) -> Result<Self>
	where
		R: Read + Seek,
		Self: Sized,
	{
		crate::probe::Probe::new(reader)
			.guess_file_type()?
			.options(parse_options)
			.read()
	}

	fn save_to(&self, file: &mut File, write_options: WriteOptions) -> Result<()> {
		for tag in &self.tags {
			// TODO: This is a temporary solution. Ideally we should probe once and use
			//       the format-specific writing to avoid these rewinds.
			file.rewind()?;
			tag.save_to(file, write_options)?;
		}

		Ok(())
	}

	fn properties(&self) -> &Self::Properties {
		&self.properties
	}

	fn contains_tag(&self) -> bool {
		!self.tags.is_empty()
	}

	fn contains_tag_type(&self, tag_type: TagType) -> bool {
		self.tags.iter().any(|t| t.tag_type() == tag_type)
	}
}

impl From<BoundTaggedFile> for TaggedFile {
	fn from(input: BoundTaggedFile) -> Self {
		input.inner
	}
}

/// A variant of [`TaggedFile`] that holds a [`File`] handle, and reflects changes
/// such as tag removals.
///
/// For example:
///
/// ```rust,no_run
/// use lofty::config::WriteOptions;
/// use lofty::file::{AudioFile, TaggedFileExt};
/// use lofty::tag::{Tag, TagType};
/// # fn main() -> lofty::error::Result<()> {
/// # let path = "tests/files/assets/minimal/full_test.mp3";
///
/// // We create an empty tag
/// let tag = Tag::new(TagType::Id3v2);
///
/// let mut tagged_file = lofty::read_from_path(path)?;
///
/// // Push our empty tag into the TaggedFile
/// tagged_file.insert_tag(tag);
///
/// // After saving, our file still "contains" the ID3v2 tag, but if we were to read
/// // "foo.mp3", it would not have an ID3v2 tag. Lofty does not write empty tags, but this
/// // change will not be reflected in `TaggedFile`.
/// tagged_file.save_to_path("foo.mp3", WriteOptions::default())?;
/// assert!(tagged_file.contains_tag_type(TagType::Id3v2));
/// # Ok(()) }
/// ```
///
/// However, when using `BoundTaggedFile`:
///
/// ```rust,no_run
/// use lofty::config::{ParseOptions, WriteOptions};
/// use lofty::file::{AudioFile, BoundTaggedFile, TaggedFileExt};
/// use lofty::tag::{Tag, TagType};
/// use std::fs::OpenOptions;
/// # fn main() -> lofty::error::Result<()> {
/// # let path = "tests/files/assets/minimal/full_test.mp3";
///
/// // We create an empty tag
/// let tag = Tag::new(TagType::Id3v2);
///
/// // We'll need to open our file for reading *and* writing
/// let file = OpenOptions::new().read(true).write(true).open(path)?;
/// let parse_options = ParseOptions::new();
///
/// let mut bound_tagged_file = BoundTaggedFile::read_from(file, parse_options)?;
///
/// // Push our empty tag into the TaggedFile
/// bound_tagged_file.insert_tag(tag);
///
/// // Now when saving, we no longer have to specify a path, and the tags in the `BoundTaggedFile`
/// // reflect those in the actual file on disk.
/// bound_tagged_file.save(WriteOptions::default())?;
/// assert!(!bound_tagged_file.contains_tag_type(TagType::Id3v2));
/// # Ok(()) }
/// ```
pub struct BoundTaggedFile {
	inner: TaggedFile,
	file_handle: File,
}

impl BoundTaggedFile {
	/// Create a new [`BoundTaggedFile`]
	///
	/// # Errors
	///
	/// See [`AudioFile::read_from`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::config::ParseOptions;
	/// use lofty::file::{AudioFile, BoundTaggedFile, TaggedFileExt};
	/// use lofty::tag::{Tag, TagType};
	/// use std::fs::OpenOptions;
	/// # fn main() -> lofty::error::Result<()> {
	/// # let path = "tests/files/assets/minimal/full_test.mp3";
	///
	/// // We'll need to open our file for reading *and* writing
	/// let file = OpenOptions::new().read(true).write(true).open(path)?;
	/// let parse_options = ParseOptions::new();
	///
	/// let bound_tagged_file = BoundTaggedFile::read_from(file, parse_options)?;
	/// # Ok(()) }
	/// ```
	pub fn read_from(mut file: File, parse_options: ParseOptions) -> Result<Self> {
		let inner = TaggedFile::read_from(&mut file, parse_options)?;
		file.rewind()?;

		Ok(Self {
			inner,
			file_handle: file,
		})
	}

	/// Save the tags to the file stored internally
	///
	/// # Errors
	///
	/// See [`TaggedFile::save_to`]
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use lofty::config::{ParseOptions, WriteOptions};
	/// use lofty::file::{AudioFile, BoundTaggedFile, TaggedFileExt};
	/// use lofty::tag::{Tag, TagType};
	/// use std::fs::OpenOptions;
	/// # fn main() -> lofty::error::Result<()> {
	/// # let path = "tests/files/assets/minimal/full_test.mp3";
	///
	/// // We'll need to open our file for reading *and* writing
	/// let file = OpenOptions::new().read(true).write(true).open(path)?;
	/// let parse_options = ParseOptions::new();
	///
	/// let mut bound_tagged_file = BoundTaggedFile::read_from(file, parse_options)?;
	///
	/// // Do some work to the tags...
	///
	/// // This will save the tags to the file we provided to `read_from`
	/// bound_tagged_file.save(WriteOptions::default())?;
	/// # Ok(()) }
	/// ```
	pub fn save(&mut self, write_options: WriteOptions) -> Result<()> {
		self.inner.save_to(&mut self.file_handle, write_options)?;
		self.inner.tags.retain(|tag| !tag.is_empty());

		Ok(())
	}
}

impl TaggedFileExt for BoundTaggedFile {
	fn file_type(&self) -> FileType {
		self.inner.file_type()
	}

	fn tags(&self) -> &[Tag] {
		self.inner.tags()
	}

	fn tag(&self, tag_type: TagType) -> Option<&Tag> {
		self.inner.tag(tag_type)
	}

	fn tag_mut(&mut self, tag_type: TagType) -> Option<&mut Tag> {
		self.inner.tag_mut(tag_type)
	}

	fn first_tag_mut(&mut self) -> Option<&mut Tag> {
		self.inner.first_tag_mut()
	}

	fn insert_tag(&mut self, tag: Tag) -> Option<Tag> {
		self.inner.insert_tag(tag)
	}

	fn remove(&mut self, tag_type: TagType) -> Option<Tag> {
		self.inner.remove(tag_type)
	}

	fn clear(&mut self) {
		self.inner.clear()
	}
}

impl AudioFile for BoundTaggedFile {
	type Properties = FileProperties;

	fn read_from<R>(_: &mut R, _: ParseOptions) -> Result<Self>
	where
		R: Read + Seek,
		Self: Sized,
	{
		unimplemented!(
			"BoundTaggedFile can only be constructed through `BoundTaggedFile::read_from`"
		)
	}

	fn save_to(&self, file: &mut File, write_options: WriteOptions) -> Result<()> {
		self.inner.save_to(file, write_options)
	}

	fn properties(&self) -> &Self::Properties {
		self.inner.properties()
	}

	fn contains_tag(&self) -> bool {
		self.inner.contains_tag()
	}

	fn contains_tag_type(&self, tag_type: TagType) -> bool {
		self.inner.contains_tag_type(tag_type)
	}
}
