use crate::error::Result;
use crate::properties::FileProperties;
use crate::tag::{Tag, TagType};
use crate::traits::TagExt;

use crate::resolve::CUSTOM_RESOLVERS;
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
	fn read_from<R>(reader: &mut R, read_properties: bool) -> Result<Self>
	where
		R: Read + Seek,
		Self: Sized;
	/// Returns a reference to the file's properties
	fn properties(&self) -> &Self::Properties;
	/// Checks if the file contains any tags
	fn contains_tag(&self) -> bool;
	/// Checks if the file contains the given [`TagType`]
	fn contains_tag_type(&self, tag_type: TagType) -> bool;
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
	pub fn new(ty: FileType, properties: FileProperties, tags: Vec<Tag>) -> Self {
		Self {
			ty,
			properties,
			tags,
		}
	}

	/// Returns the file's [`FileType`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::FileType;
	///
	/// # fn main() -> lofty::Result<()> {
	/// # let path_to_mp3 = "tests/files/assets/minimal/full_test.mp3";
	/// let mut tagged_file = lofty::read_from_path(path_to_mp3, true)?;
	///
	/// assert_eq!(tagged_file.file_type(), FileType::MPEG);
	/// # Ok(()) }
	/// ```
	pub fn file_type(&self) -> FileType {
		self.ty
	}

	/// Returns all tags
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::FileType;
	///
	/// # fn main() -> lofty::Result<()> {
	/// # let path_to_mp3 = "tests/files/assets/minimal/full_test.mp3";
	/// // An MP3 file with 3 tags
	/// let mut tagged_file = lofty::read_from_path(path_to_mp3, true)?;
	///
	/// let tags = tagged_file.tags();
	///
	/// assert_eq!(tags.len(), 3);
	/// # Ok(()) }
	/// ```
	pub fn tags(&self) -> &[Tag] {
		self.tags.as_slice()
	}

	/// Returns the file type's primary [`TagType`]
	///
	/// See [`FileType::primary_tag_type`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::TagType;
	///
	/// # fn main() -> lofty::Result<()> {
	/// # let path_to_mp3 = "tests/files/assets/minimal/full_test.mp3";
	/// let mut tagged_file = lofty::read_from_path(path_to_mp3, true)?;
	///
	/// assert_eq!(tagged_file.primary_tag_type(), TagType::ID3v2);
	/// # Ok(()) }
	/// ```
	pub fn primary_tag_type(&self) -> TagType {
		self.ty.primary_tag_type()
	}

	/// Determines whether the file supports the given [`TagType`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::TagType;
	///
	/// # fn main() -> lofty::Result<()> {
	/// # let path_to_mp3 = "tests/files/assets/minimal/full_test.mp3";
	/// let mut tagged_file = lofty::read_from_path(path_to_mp3, true)?;
	///
	/// assert!(tagged_file.supports_tag_type(TagType::ID3v2));
	/// # Ok(()) }
	/// ```
	pub fn supports_tag_type(&self, tag_type: TagType) -> bool {
		self.ty.supports_tag_type(tag_type)
	}

	/// Get a reference to a specific [`TagType`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::TagType;
	///
	/// # fn main() -> lofty::Result<()> {
	/// # let path_to_mp3 = "tests/files/assets/minimal/full_test.mp3";
	/// // Read an MP3 file with an ID3v2 tag
	/// let mut tagged_file = lofty::read_from_path(path_to_mp3, true)?;
	///
	/// // An ID3v2 tag
	/// let tag = tagged_file.tag(TagType::ID3v2);
	///
	/// assert!(tag.is_some());
	/// assert_eq!(tag.unwrap().tag_type(), TagType::ID3v2);
	/// # Ok(()) }
	/// ```
	pub fn tag(&self, tag_type: TagType) -> Option<&Tag> {
		self.tags.iter().find(|i| i.tag_type() == tag_type)
	}

	/// Get a mutable reference to a specific [`TagType`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::TagType;
	///
	/// # fn main() -> lofty::Result<()> {
	/// # let path_to_mp3 = "tests/files/assets/minimal/full_test.mp3";
	/// // Read an MP3 file with an ID3v2 tag
	/// let mut tagged_file = lofty::read_from_path(path_to_mp3, true)?;
	///
	/// // An ID3v2 tag
	/// let tag = tagged_file.tag(TagType::ID3v2);
	///
	/// assert!(tag.is_some());
	/// assert_eq!(tag.unwrap().tag_type(), TagType::ID3v2);
	///
	/// // Alter the tag...
	/// # Ok(()) }
	/// ```
	pub fn tag_mut(&mut self, tag_type: TagType) -> Option<&mut Tag> {
		self.tags.iter_mut().find(|i| i.tag_type() == tag_type)
	}

	/// Returns the primary tag
	///
	/// See [`FileType::primary_tag_type`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::TagType;
	///
	/// # fn main() -> lofty::Result<()> {
	/// # let path_to_mp3 = "tests/files/assets/minimal/full_test.mp3";
	/// // Read an MP3 file with an ID3v2 tag
	/// let mut tagged_file = lofty::read_from_path(path_to_mp3, true)?;
	///
	/// // An ID3v2 tag
	/// let tag = tagged_file.primary_tag();
	///
	/// assert!(tag.is_some());
	/// assert_eq!(tag.unwrap().tag_type(), TagType::ID3v2);
	/// # Ok(()) }
	/// ```
	pub fn primary_tag(&self) -> Option<&Tag> {
		self.tag(self.primary_tag_type())
	}

	/// Gets a mutable reference to the file's "Primary tag"
	///
	/// See [`FileType::primary_tag_type`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::TagType;
	///
	/// # fn main() -> lofty::Result<()> {
	/// # let path_to_mp3 = "tests/files/assets/minimal/full_test.mp3";
	/// // Read an MP3 file with an ID3v2 tag
	/// let mut tagged_file = lofty::read_from_path(path_to_mp3, true)?;
	///
	/// // An ID3v2 tag
	/// let tag = tagged_file.primary_tag_mut();
	///
	/// assert!(tag.is_some());
	/// assert_eq!(tag.unwrap().tag_type(), TagType::ID3v2);
	///
	/// // Alter the tag...
	/// # Ok(()) }
	/// ```
	pub fn primary_tag_mut(&mut self) -> Option<&mut Tag> {
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
	/// # fn main() -> lofty::Result<()> {
	/// # let path = "tests/files/assets/minimal/full_test.mp3";
	/// // A file we know has tags
	/// let mut tagged_file = lofty::read_from_path(path, true)?;
	///
	/// // A tag of a (currently) unknown type
	/// let tag = tagged_file.first_tag();
	/// assert!(tag.is_some());
	/// # Ok(()) }
	/// ```
	pub fn first_tag(&self) -> Option<&Tag> {
		self.tags.first()
	}

	/// Gets a mutable reference to the first tag, if there are any
	///
	/// NOTE: This will grab the first available tag, you cannot rely on the result being
	/// a specific type
	///
	/// # Examples
	///
	/// ```rust
	/// # fn main() -> lofty::Result<()> {
	/// # let path = "tests/files/assets/minimal/full_test.mp3";
	/// // A file we know has tags
	/// let mut tagged_file = lofty::read_from_path(path, true)?;
	///
	/// // A tag of a (currently) unknown type
	/// let tag = tagged_file.first_tag_mut();
	/// assert!(tag.is_some());
	///
	/// // Alter the tag...
	/// # Ok(()) }
	/// ```
	pub fn first_tag_mut(&mut self) -> Option<&mut Tag> {
		self.tags.first_mut()
	}

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
	/// use lofty::{AudioFile, Tag, TagType};
	///
	/// # fn main() -> lofty::Result<()> {
	/// # let path_to_mp3 = "tests/files/assets/minimal/full_test.mp3";
	/// // Read an MP3 file without an ID3v2 tag
	/// let mut tagged_file = lofty::read_from_path(path_to_mp3, true)?;
	/// # let _ = tagged_file.take(TagType::ID3v2); // sneaky
	///
	/// assert!(!tagged_file.contains_tag_type(TagType::ID3v2));
	///
	/// // Insert the ID3v2 tag
	/// let new_id3v2_tag = Tag::new(TagType::ID3v2);
	/// tagged_file.insert_tag(new_id3v2_tag);
	///
	/// assert!(tagged_file.contains_tag_type(TagType::ID3v2));
	/// # Ok(()) }
	/// ```
	pub fn insert_tag(&mut self, tag: Tag) -> Option<Tag> {
		let tag_type = tag.tag_type();

		if self.supports_tag_type(tag_type) {
			let ret = self.take(tag_type);
			self.tags.push(tag);

			return ret;
		}

		None
	}

	/// Removes a specific [`TagType`] and returns it
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::{AudioFile, TagType};
	///
	/// # fn main() -> lofty::Result<()> {
	/// # let path_to_mp3 = "tests/files/assets/minimal/full_test.mp3";
	/// // Read an MP3 file containing an ID3v2 tag
	/// let mut tagged_file = lofty::read_from_path(path_to_mp3, true)?;
	///
	/// assert!(tagged_file.contains_tag_type(TagType::ID3v2));
	///
	/// // Take the ID3v2 tag
	/// let id3v2 = tagged_file.take(TagType::ID3v2);
	///
	/// assert!(!tagged_file.contains_tag_type(TagType::ID3v2));
	/// # Ok(()) }
	/// ```
	pub fn take(&mut self, tag_type: TagType) -> Option<Tag> {
		self.tags
			.iter()
			.position(|t| t.tag_type() == tag_type)
			.map(|pos| self.tags.remove(pos))
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
	/// use lofty::{AudioFile, FileType, TagType};
	///
	/// # fn main() -> lofty::Result<()> {
	/// # let path_to_mp3 = "tests/files/assets/minimal/full_test.mp3";
	/// // Read an MP3 file containing an ID3v2 tag
	/// let mut tagged_file = lofty::read_from_path(path_to_mp3, true)?;
	///
	/// assert!(tagged_file.contains_tag_type(TagType::ID3v2));
	///
	/// // Remap our MP3 file to WavPack, which doesn't support ID3v2
	/// tagged_file.change_file_type(FileType::WavPack);
	///
	/// assert!(!tagged_file.contains_tag_type(TagType::ID3v2));
	/// # Ok(()) }
	/// ```
	pub fn change_file_type(&mut self, file_type: FileType) {
		self.ty = file_type;
		self.properties = FileProperties::default();
		self.tags
			.retain(|t| self.ty.supports_tag_type(t.tag_type()));
	}

	/// Removes all tags from the file
	///
	/// # Examples
	///
	/// ```rust
	/// # fn main() -> lofty::Result<()> {
	/// # let path = "tests/files/assets/minimal/full_test.mp3";
	/// let mut tagged_file = lofty::read_from_path(path, true)?;
	///
	/// tagged_file.clear();
	///
	/// assert!(tagged_file.tags().is_empty());
	/// # Ok(()) }
	/// ```
	pub fn clear(&mut self) {
		self.tags.clear()
	}

	/// Attempts to write all tags to a path
	///
	/// # Errors
	///
	/// See [`TaggedFile::save_to`]
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// # fn main() -> lofty::Result<()> {
	/// # let path = "tests/files/assets/minimal/full_test.mp3";
	/// let mut tagged_file = lofty::read_from_path(path, true)?;
	///
	/// // Edit the tags
	///
	/// tagged_file.save_to_path(path)?;
	/// # Ok(()) }
	/// ```
	pub fn save_to_path(&self, path: impl AsRef<Path>) -> Result<()> {
		self.save_to(&mut OpenOptions::new().read(true).write(true).open(path)?)
	}

	/// Attempts to write all tags to a file
	///
	/// # Errors
	///
	/// See [`Tag::save_to`], however this is applicable to every tag in the `TaggedFile`.
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use std::fs::OpenOptions;
	///
	/// # fn main() -> lofty::Result<()> {
	/// # let path = "tests/files/assets/minimal/full_test.mp3";
	/// let mut tagged_file = lofty::read_from_path(path, true)?;
	///
	/// // Edit the tags
	///
	/// let mut file = OpenOptions::new().read(true).write(true).open(path)?;
	/// tagged_file.save_to(&mut file)?;
	/// # Ok(()) }
	/// ```
	pub fn save_to(&self, file: &mut File) -> Result<()> {
		for tag in &self.tags {
			tag.save_to(file)?;
		}

		Ok(())
	}
}

impl AudioFile for TaggedFile {
	type Properties = FileProperties;

	fn read_from<R>(reader: &mut R, read_properties: bool) -> Result<Self>
	where
		R: Read + Seek,
		Self: Sized,
	{
		crate::probe::Probe::new(reader)
			.guess_file_type()?
			.read(read_properties)
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

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
#[allow(missing_docs)]
#[non_exhaustive]
/// The type of file read
pub enum FileType {
	AIFF,
	APE,
	FLAC,
	MPEG,
	MP4,
	Opus,
	Vorbis,
	Speex,
	WAV,
	WavPack,
	Custom(&'static str),
}

impl FileType {
	#[allow(unreachable_patterns, clippy::match_same_arms)]
	/// Returns the file type's "primary" [`TagType`], or the one most likely to be used in the target format
	///
	/// | [`FileType`]             | [`TagType`]      |
	/// |--------------------------|------------------|
	/// | `AIFF`, `MP3`, `WAV`     | `Id3v2`          |
	/// | `APE` , `WavPack`        | `Ape`            |
	/// | `FLAC`, `Opus`, `Vorbis` | `VorbisComments` |
	/// | `MP4`                    | `Mp4Ilst`        |
	///
	/// # Panics
	///
	/// If an unregistered `FileType` ([`FileType::Custom`]) is encountered. See [`crate::resolve::register_custom_resolver`].
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::{FileType, TagType};
	///
	/// let file_type = FileType::MPEG;
	/// assert_eq!(file_type.primary_tag_type(), TagType::ID3v2);
	/// ```
	pub fn primary_tag_type(&self) -> TagType {
		match self {
			#[cfg(all(not(feature = "id3v2"), feature = "aiff_text_chunks"))]
			FileType::AIFF => TagType::AIFFText,
			#[cfg(all(not(feature = "id3v2"), feature = "riff_info_list"))]
			FileType::WAV => TagType::RIFFInfo,
			#[cfg(all(not(feature = "id3v2"), feature = "id3v1"))]
			FileType::MPEG => TagType::ID3v1,
			#[cfg(all(not(feature = "id3v2"), not(feature = "id3v1"), feature = "ape"))]
			FileType::MPEG => TagType::APE,
			FileType::AIFF | FileType::MPEG | FileType::WAV => TagType::ID3v2,
			#[cfg(all(not(feature = "ape"), feature = "id3v1"))]
			FileType::MPEG | FileType::WavPack => TagType::ID3v1,
			FileType::APE | FileType::WavPack => TagType::APE,
			FileType::FLAC | FileType::Opus | FileType::Vorbis | FileType::Speex => {
				TagType::VorbisComments
			},
			FileType::MP4 => TagType::MP4ilst,
			FileType::Custom(c) => {
				if let Some(r) = crate::resolve::lookup_resolver(c) {
					r.primary_tag_type()
				} else {
					panic!(
						"Encountered an unregistered custom `FileType` named `{}`",
						c
					);
				}
			},
		}
	}

	/// Returns if the target `FileType` supports a [`TagType`]
	///
	/// # Panics
	///
	/// If an unregistered `FileType` ([`FileType::Custom`]) is encountered. See [`crate::resolve::register_custom_resolver`].
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::{FileType, TagType};
	///
	/// let file_type = FileType::MPEG;
	/// assert!(file_type.supports_tag_type(TagType::ID3v2));
	/// ```
	pub fn supports_tag_type(&self, tag_type: TagType) -> bool {
		match self {
			#[cfg(feature = "id3v2")]
			FileType::AIFF | FileType::APE | FileType::MPEG | FileType::WAV
				if tag_type == TagType::ID3v2 =>
			{
				true
			},
			#[cfg(feature = "aiff_text_chunks")]
			FileType::AIFF if tag_type == TagType::AIFFText => true,
			#[cfg(feature = "id3v1")]
			FileType::APE | FileType::MPEG | FileType::WavPack if tag_type == TagType::ID3v1 => true,
			#[cfg(feature = "ape")]
			FileType::APE | FileType::MPEG | FileType::WavPack if tag_type == TagType::APE => true,
			#[cfg(feature = "vorbis_comments")]
			FileType::Opus | FileType::FLAC | FileType::Vorbis | FileType::Speex => {
				tag_type == TagType::VorbisComments
			},
			#[cfg(feature = "mp4_ilst")]
			FileType::MP4 => tag_type == TagType::MP4ilst,
			#[cfg(feature = "riff_info_list")]
			FileType::WAV => tag_type == TagType::RIFFInfo,
			FileType::Custom(c) => {
				if let Some(r) = crate::resolve::lookup_resolver(c) {
					r.supported_tag_types().contains(&tag_type)
				} else {
					panic!(
						"Encountered an unregistered custom `FileType` named `{}`",
						c
					);
				}
			},
			_ => false,
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
	/// assert_eq!(FileType::from_ext(extension), Some(FileType::MPEG));
	/// ```
	pub fn from_ext<E>(ext: E) -> Option<Self>
	where
		E: AsRef<OsStr>,
	{
		let ext = ext.as_ref().to_str()?.to_ascii_lowercase();

		match ext.as_str() {
			"ape" => Some(Self::APE),
			"aiff" | "aif" | "afc" | "aifc" => Some(Self::AIFF),
			"mp3" | "mp2" | "mp1" => Some(Self::MPEG),
			"wav" | "wave" => Some(Self::WAV),
			"wv" => Some(Self::WavPack),
			"opus" => Some(Self::Opus),
			"flac" => Some(Self::FLAC),
			"ogg" => Some(Self::Vorbis),
			"mp4" | "m4a" | "m4b" | "m4p" | "m4r" | "m4v" | "3gp" => Some(Self::MP4),
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
	/// assert_eq!(FileType::from_path(path), Some(FileType::MPEG));
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
			(Some(f_ty), _) => Some(f_ty),
			// We make no attempt to search past an ID3v2 tag here, since
			// we only provided a fixed-sized buffer to search from.
			//
			// That case is handled in `Probe::guess_file_type`
			_ => None,
		}
	}

	// TODO: APE tags in the beginning of the file
	pub(crate) fn from_buffer_inner(buf: &[u8]) -> (Option<Self>, Option<u32>) {
		use crate::id3::v2::unsynch_u32;

		// Start out with an empty return: (File type, id3 size)
		// Only one can be set
		let mut ret = (None, None);

		if buf.is_empty() {
			return ret;
		}

		match Self::quick_type_guess(buf) {
			Some(f_ty) => ret.0 = Some(f_ty),
			// Special case for ID3, gets checked in `Probe::guess_file_type`
			// The bare minimum size for an ID3v2 header is 10 bytes
			None if buf.len() >= 10 && &buf[..3] == b"ID3" => {
				// This is infallible, but preferable to an unwrap
				if let Ok(arr) = buf[6..10].try_into() {
					// Set the ID3v2 size
					ret.1 = Some(unsynch_u32(u32::from_be_bytes(arr)));
				}
			},
			// We aren't able to determine a format
			_ => {},
		}

		ret
	}

	fn quick_type_guess(buf: &[u8]) -> Option<Self> {
		use crate::mp3::header::verify_frame_sync;

		// Safe to index, since we return early on an empty buffer
		match buf[0] {
			77 if buf.starts_with(b"MAC") => Some(Self::APE),
			255 if buf.len() >= 2 && verify_frame_sync([buf[0], buf[1]]) => Some(Self::MPEG),
			70 if buf.len() >= 12 && &buf[..4] == b"FORM" => {
				let id = &buf[8..12];

				if id == b"AIFF" || id == b"AIFC" {
					return Some(Self::AIFF);
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
			102 if buf.starts_with(b"fLaC") => Some(Self::FLAC),
			82 if buf.len() >= 12 && &buf[..4] == b"RIFF" => {
				if &buf[8..12] == b"WAVE" {
					return Some(Self::WAV);
				}

				None
			},
			119 if buf.len() >= 4 && &buf[..4] == b"wvpk" => Some(Self::WavPack),
			_ if buf.len() >= 8 && &buf[4..8] == b"ftyp" => Some(Self::MP4),
			_ => None,
		}
	}
}
