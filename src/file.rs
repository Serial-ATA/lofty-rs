use crate::error::Result;
use crate::probe::ParseOptions;
use crate::properties::FileProperties;
use crate::resolve::CUSTOM_RESOLVERS;
use crate::tag::{Tag, TagType};
use crate::traits::TagExt;

use std::convert::TryInto;
use std::ffi::OsStr;
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
	/// use lofty::{AudioFile, TaggedFileExt};
	///
	/// # fn main() -> lofty::Result<()> {
	/// # let path = "tests/files/assets/minimal/full_test.mp3";
	/// let mut tagged_file = lofty::read_from_path(path)?;
	///
	/// // Edit the tags
	///
	/// tagged_file.save_to_path(path)?;
	/// # Ok(()) }
	/// ```
	fn save_to_path(&self, path: impl AsRef<Path>) -> Result<()> {
		self.save_to(&mut OpenOptions::new().read(true).write(true).open(path)?)
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
	/// use lofty::{AudioFile, TaggedFileExt};
	/// use std::fs::OpenOptions;
	///
	/// # fn main() -> lofty::Result<()> {
	/// # let path = "tests/files/assets/minimal/full_test.mp3";
	/// let mut tagged_file = lofty::read_from_path(path)?;
	///
	/// // Edit the tags
	///
	/// let mut file = OpenOptions::new().read(true).write(true).open(path)?;
	/// tagged_file.save_to(&mut file)?;
	/// # Ok(()) }
	/// ```
	fn save_to(&self, file: &mut File) -> Result<()>;

	/// Returns a reference to the file's properties
	fn properties(&self) -> &Self::Properties;
	/// Checks if the file contains any tags
	fn contains_tag(&self) -> bool;
	/// Checks if the file contains the given [`TagType`]
	fn contains_tag_type(&self, tag_type: TagType) -> bool;
}

/// Provides a common interface between [`TaggedFile`] and [`BoundTaggedFile`]
pub trait TaggedFileExt {
	/// Returns the file's [`FileType`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::{FileType, TaggedFileExt};
	///
	/// # fn main() -> lofty::Result<()> {
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
	/// use lofty::{FileType, TaggedFileExt};
	///
	/// # fn main() -> lofty::Result<()> {
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
	/// use lofty::{TagType, TaggedFileExt};
	///
	/// # fn main() -> lofty::Result<()> {
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
	/// use lofty::{TagType, TaggedFileExt};
	///
	/// # fn main() -> lofty::Result<()> {
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
	/// use lofty::{TagType, TaggedFileExt};
	///
	/// # fn main() -> lofty::Result<()> {
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
	/// use lofty::{TagType, TaggedFileExt};
	///
	/// # fn main() -> lofty::Result<()> {
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
	/// use lofty::{TagType, TaggedFileExt};
	///
	/// # fn main() -> lofty::Result<()> {
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
	/// use lofty::{TagType, TaggedFileExt};
	///
	/// # fn main() -> lofty::Result<()> {
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
	/// use lofty::TaggedFileExt;
	///
	/// # fn main() -> lofty::Result<()> {
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
	/// use lofty::TaggedFileExt;
	///
	/// # fn main() -> lofty::Result<()> {
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
	/// use lofty::{AudioFile, Tag, TagType, TaggedFileExt};
	///
	/// # fn main() -> lofty::Result<()> {
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
	/// use lofty::{AudioFile, TagType, TaggedFileExt};
	///
	/// # fn main() -> lofty::Result<()> {
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
	/// use lofty::TaggedFileExt;
	///
	/// # fn main() -> lofty::Result<()> {
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
	/// use lofty::{AudioFile, FileType, TagType, TaggedFileExt};
	///
	/// # fn main() -> lofty::Result<()> {
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

	fn save_to(&self, file: &mut File) -> Result<()> {
		for tag in &self.tags {
			// TODO: This is a temporary solution. Ideally we should probe once and use
			//       the format-specific writing to avoid these rewinds.
			file.rewind()?;
			tag.save_to(file)?;
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
/// use lofty::{AudioFile, Tag, TagType, TaggedFileExt};
/// # fn main() -> lofty::Result<()> {
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
/// tagged_file.save_to_path("foo.mp3")?;
/// assert!(tagged_file.contains_tag_type(TagType::Id3v2));
/// # Ok(()) }
/// ```
///
/// However, when using `BoundTaggedFile`:
///
/// ```rust,no_run
/// use lofty::{AudioFile, BoundTaggedFile, ParseOptions, Tag, TagType, TaggedFileExt};
/// use std::fs::OpenOptions;
/// # fn main() -> lofty::Result<()> {
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
/// bound_tagged_file.save()?;
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
	/// use lofty::{AudioFile, BoundTaggedFile, ParseOptions, Tag, TagType, TaggedFileExt};
	/// use std::fs::OpenOptions;
	/// # fn main() -> lofty::Result<()> {
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
	/// use lofty::{AudioFile, BoundTaggedFile, ParseOptions, Tag, TagType, TaggedFileExt};
	/// use std::fs::OpenOptions;
	/// # fn main() -> lofty::Result<()> {
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
	/// bound_tagged_file.save()?;
	/// # Ok(()) }
	/// ```
	pub fn save(&mut self) -> Result<()> {
		self.inner.save_to(&mut self.file_handle)?;
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

	fn save_to(&self, file: &mut File) -> Result<()> {
		self.inner.save_to(file)
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

/// The type of file read
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
#[allow(missing_docs)]
#[non_exhaustive]
pub enum FileType {
	Aac,
	Aiff,
	Ape,
	Ebml,
	Flac,
	Mpeg,
	Mp4,
	Mpc,
	Opus,
	Vorbis,
	Speex,
	Wav,
	WavPack,
	Custom(&'static str),
}

impl FileType {
	/// Returns the file type's "primary" [`TagType`], or the one most likely to be used in the target format
	///
	/// | [`FileType`]                      | [`TagType`]      |
	/// |-----------------------------------|------------------|
	/// | `Aac`, `Aiff`, `Mp3`, `Wav`       | `Id3v2`          |
	/// | `Ape` , `Mpc`, `WavPack`          | `Ape`            |
	/// | `Flac`, `Opus`, `Vorbis`, `Speex` | `VorbisComments` |
	/// | `Mp4`                             | `Mp4Ilst`        |
	///
	/// # Panics
	///
	/// If an unregistered `FileType` ([`FileType::Custom`]) is encountered. See [`register_custom_resolver`](crate::resolve::register_custom_resolver).
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::{FileType, TagType};
	///
	/// let file_type = FileType::Mpeg;
	/// assert_eq!(file_type.primary_tag_type(), TagType::Id3v2);
	/// ```
	pub fn primary_tag_type(&self) -> TagType {
		match self {
			FileType::Aac | FileType::Aiff | FileType::Mpeg | FileType::Wav => TagType::Id3v2,
			FileType::Ape | FileType::Mpc | FileType::WavPack => TagType::Ape,
			FileType::Ebml => TagType::Ebml,
			FileType::Flac | FileType::Opus | FileType::Vorbis | FileType::Speex => {
				TagType::VorbisComments
			},
			FileType::Mp4 => TagType::Mp4Ilst,
			FileType::Custom(c) => {
				let resolver = crate::resolve::lookup_resolver(c);
				resolver.primary_tag_type()
			},
		}
	}

	/// Returns if the target `FileType` supports a [`TagType`]
	///
	/// NOTE: This is feature dependent, meaning if you do not have the
	///       `id3v2` feature enabled, [`FileType::Mpeg`] will return `false` for
	///        [`TagType::Id3v2`].
	///
	/// # Panics
	///
	/// If an unregistered `FileType` ([`FileType::Custom`]) is encountered. See [`register_custom_resolver`](crate::resolve::register_custom_resolver).
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::{FileType, TagType};
	///
	/// let file_type = FileType::Mpeg;
	/// assert!(file_type.supports_tag_type(TagType::Id3v2));
	/// ```
	pub fn supports_tag_type(&self, tag_type: TagType) -> bool {
		if let FileType::Custom(c) = self {
			let resolver = crate::resolve::lookup_resolver(c);
			return resolver.supported_tag_types().contains(&tag_type);
		}

		match tag_type {
			TagType::AiffText => crate::iff::aiff::AIFFTextChunks::SUPPORTED_FORMATS.contains(self),
			TagType::Ape => crate::ape::ApeTag::SUPPORTED_FORMATS.contains(self),
			TagType::Ebml => crate::ebml::EbmlTag::SUPPORTED_FORMATS.contains(self),
			TagType::Id3v1 => crate::id3::v1::Id3v1Tag::SUPPORTED_FORMATS.contains(self),
			TagType::Id3v2 => crate::id3::v2::Id3v2Tag::SUPPORTED_FORMATS.contains(self),
			TagType::Mp4Ilst => crate::mp4::Ilst::SUPPORTED_FORMATS.contains(self),
			TagType::RiffInfo => crate::iff::wav::RIFFInfoList::SUPPORTED_FORMATS.contains(self),
			TagType::VorbisComments => crate::ogg::VorbisComments::SUPPORTED_FORMATS.contains(self),
		}
	}

	/// Attempts to extract a [`FileType`] from an extension
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::FileType;
	///
	/// let extension = "mp3";
	/// assert_eq!(FileType::from_ext(extension), Some(FileType::Mpeg));
	/// ```
	pub fn from_ext<E>(ext: E) -> Option<Self>
	where
		E: AsRef<OsStr>,
	{
		let ext = ext.as_ref().to_str()?.to_ascii_lowercase();

		match ext.as_str() {
			"aac" => Some(Self::Aac),
			"ape" => Some(Self::Ape),
			"aiff" | "aif" | "afc" | "aifc" => Some(Self::Aiff),
			"mp3" | "mp2" | "mp1" => Some(Self::Mpeg),
			"wav" | "wave" => Some(Self::Wav),
			"wv" => Some(Self::WavPack),
			"opus" => Some(Self::Opus),
			"flac" => Some(Self::Flac),
			"ogg" => Some(Self::Vorbis),
			"mka" | "mkv" | "webm" => Some(Self::Ebml),
			"mp4" | "m4a" | "m4b" | "m4p" | "m4r" | "m4v" | "3gp" => Some(Self::Mp4),
			"mpc" | "mp+" | "mpp" => Some(Self::Mpc),
			"spx" => Some(Self::Speex),
			e => {
				if let Some((ty, _)) = CUSTOM_RESOLVERS
					.lock()
					.ok()?
					.iter()
					.find(|(_, f)| f.extension() == Some(e))
				{
					Some(Self::Custom(ty))
				} else {
					None
				}
			},
		}
	}

	/// Attempts to determine a [`FileType`] from a path
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::FileType;
	/// use std::path::Path;
	///
	/// let path = Path::new("path/to/my.mp3");
	/// assert_eq!(FileType::from_path(path), Some(FileType::Mpeg));
	/// ```
	pub fn from_path<P>(path: P) -> Option<Self>
	where
		P: AsRef<Path>,
	{
		let ext = path.as_ref().extension();
		ext.and_then(Self::from_ext)
	}

	/// Attempts to extract a [`FileType`] from a buffer
	///
	/// NOTES:
	///
	/// * This is for use in [`Probe::guess_file_type`], it
	/// is recommended to use it that way
	/// * This **will not** search past tags at the start of the buffer.
	/// For this behavior, use [`Probe::guess_file_type`].
	///
	/// [`Probe::guess_file_type`]: crate::Probe::guess_file_type
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::FileType;
	/// use std::fs::File;
	/// use std::io::Read;
	///
	/// # fn main() -> lofty::Result<()> {
	/// # let path_to_opus = "tests/files/assets/minimal/full_test.opus";
	/// let mut file = File::open(path_to_opus)?;
	///
	/// let mut buf = [0; 50]; // Search the first 50 bytes of the file
	/// file.read_exact(&mut buf)?;
	///
	/// assert_eq!(FileType::from_buffer(&buf), Some(FileType::Opus));
	/// # Ok(()) }
	/// ```
	pub fn from_buffer(buf: &[u8]) -> Option<Self> {
		match Self::from_buffer_inner(buf) {
			FileTypeGuessResult::Determined(file_ty) => Some(file_ty),
			// We make no attempt to search past an ID3v2 tag or junk here, since
			// we only provided a fixed-sized buffer to search from.
			//
			// That case is handled in `Probe::guess_file_type`
			_ => None,
		}
	}

	// TODO: APE tags in the beginning of the file
	pub(crate) fn from_buffer_inner(buf: &[u8]) -> FileTypeGuessResult {
		use crate::id3::v2::util::synchsafe::SynchsafeInteger;

		// Start out with an empty return
		let mut ret = FileTypeGuessResult::Undetermined;

		if buf.is_empty() {
			return ret;
		}

		match Self::quick_type_guess(buf) {
			Some(f_ty) => ret = FileTypeGuessResult::Determined(f_ty),
			// Special case for ID3, gets checked in `Probe::guess_file_type`
			// The bare minimum size for an ID3v2 header is 10 bytes
			None if buf.len() >= 10 && &buf[..3] == b"ID3" => {
				// This is infallible, but preferable to an unwrap
				if let Ok(arr) = buf[6..10].try_into() {
					// Set the ID3v2 size
					ret =
						FileTypeGuessResult::MaybePrecededById3(u32::from_be_bytes(arr).unsynch());
				}
			},
			None if buf.first().copied() == Some(0) => {
				ret = FileTypeGuessResult::MaybePrecededByJunk
			},
			// We aren't able to determine a format
			_ => {},
		}

		ret
	}

	fn quick_type_guess(buf: &[u8]) -> Option<Self> {
		use crate::mpeg::header::verify_frame_sync;

		// Safe to index, since we return early on an empty buffer
		match buf[0] {
			77 if buf.starts_with(b"MAC") => Some(Self::Ape),
			255 if buf.len() >= 2 && verify_frame_sync([buf[0], buf[1]]) => {
				// ADTS and MPEG frame headers are way too similar

				// ADTS (https://wiki.multimedia.cx/index.php/ADTS#Header):
				//
				// AAAAAAAA AAAABCCX
				//
				// Letter 	Length (bits) 	Description
				// A 	    12 	            Syncword, all bits must be set to 1.
				// B 	    1 	            MPEG Version, set to 0 for MPEG-4 and 1 for MPEG-2.
				// C 	    2 	            Layer, always set to 0.

				// MPEG (http://www.mp3-tech.org/programmer/frame_header.html):
				//
				// AAAAAAAA AAABBCCX
				//
				// Letter 	Length (bits) 	Description
				// A 	    11              Syncword, all bits must be set to 1.
				// B 	    2 	            MPEG Audio version ID
				// C 	    2 	            Layer description

				// The subtle overlap in the ADTS header's frame sync and MPEG's version ID
				// is the first condition to check. However, since 0b10 and 0b11 are valid versions
				// in MPEG, we have to also check the layer.

				// So, if we have a version 1 (0b11) or version 2 (0b10) MPEG frame AND a layer of 0b00,
				// we can assume we have an ADTS header. Awesome!

				if buf[1] & 0b10000 > 0 && buf[1] & 0b110 == 0 {
					return Some(Self::Aac);
				}

				Some(Self::Mpeg)
			},
			70 if buf.len() >= 12 && &buf[..4] == b"FORM" => {
				let id = &buf[8..12];

				if id == b"AIFF" || id == b"AIFC" {
					return Some(Self::Aiff);
				}

				None
			},
			79 if buf.len() >= 36 && &buf[..4] == b"OggS" => {
				if &buf[29..35] == b"vorbis" {
					return Some(Self::Vorbis);
				} else if &buf[28..36] == b"OpusHead" {
					return Some(Self::Opus);
				} else if &buf[28..36] == b"Speex   " {
					return Some(Self::Speex);
				}

				None
			},
			102 if buf.starts_with(b"fLaC") => Some(Self::Flac),
			82 if buf.len() >= 12 && &buf[..4] == b"RIFF" => {
				if &buf[8..12] == b"WAVE" {
					return Some(Self::Wav);
				}

				None
			},
			119 if buf.len() >= 4 && &buf[..4] == b"wvpk" => Some(Self::WavPack),
			26 if buf.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) => Some(Self::Ebml),
			_ if buf.len() >= 8 && &buf[4..8] == b"ftyp" => Some(Self::Mp4),
			_ if buf.starts_with(b"MPCK") || buf.starts_with(b"MP+") => Some(Self::Mpc),
			_ => None,
		}
	}
}

/// The result of a `FileType` guess
///
/// External callers of `FileType::from_buffer()` will only ever see `Determined` cases.
/// The remaining cases are used internally in `Probe::guess_file_type()`.
pub(crate) enum FileTypeGuessResult {
	/// The `FileType` was guessed
	Determined(FileType),
	/// The stream starts with an ID3v2 tag
	MaybePrecededById3(u32),
	/// The stream starts with junk zero bytes
	MaybePrecededByJunk,
	/// The `FileType` could not be guessed
	Undetermined,
}
