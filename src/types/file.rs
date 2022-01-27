use super::properties::FileProperties;
use super::tag::{Tag, TagType};
use crate::error::{ErrorKind, LoftyError, Result};

use std::convert::TryInto;
use std::ffi::OsStr;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek};
use std::path::Path;

/// Provides various methods for interaction with a file
pub trait AudioFile {
	/// The struct the file uses for audio properties
	///
	/// Not all formats can use [`FileProperties`] since they may contain additional information
	type Properties;

	/// Read a file from a reader
	///
	/// # Errors
	///
	/// Errors depend on the file and tags being read. See [`LoftyError`]
	fn read_from<R>(reader: &mut R, read_properties: bool) -> Result<Self>
	where
		R: Read + Seek,
		Self: Sized;
	/// Returns a reference to the file's properties
	fn properties(&self) -> &Self::Properties;
	/// Checks if the file contains any tags
	fn contains_tag(&self) -> bool;
	/// Checks if the file contains the given [`TagType`]
	fn contains_tag_type(&self, tag_type: &TagType) -> bool;
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

#[cfg(any(
	feature = "id3v1",
	feature = "riff_info_list",
	feature = "aiff_text_chunks",
	feature = "vorbis_comments",
	feature = "id3v2",
	feature = "mp4_ilst",
	feature = "ape"
))]
impl TaggedFile {
	/// Returns the primary tag
	///
	/// See [`FileType::primary_tag_type`]
	pub fn primary_tag(&self) -> Option<&Tag> {
		self.tag(&self.primary_tag_type())
	}

	/// Gets a mutable reference to the file's "Primary tag"
	///
	/// See [`FileType::primary_tag_type`]
	pub fn primary_tag_mut(&mut self) -> Option<&mut Tag> {
		self.tag_mut(&self.primary_tag_type())
	}

	/// Returns the file type's primary [`TagType`]
	///
	/// See [`FileType::primary_tag_type`]
	pub fn primary_tag_type(&self) -> TagType {
		self.ty.primary_tag_type()
	}

	/// Determines whether the file supports the given [`TagType`]
	pub fn supports_tag_type(&self, tag_type: TagType) -> bool {
		self.ty.supports_tag_type(&tag_type)
	}

	/// Returns all tags
	pub fn tags(&self) -> &[Tag] {
		self.tags.as_slice()
	}

	/// Gets the first tag, if there are any
	pub fn first_tag(&self) -> Option<&Tag> {
		self.tags.first()
	}

	/// Gets a mutable reference to the first tag, if there are any
	pub fn first_tag_mut(&mut self) -> Option<&mut Tag> {
		self.tags.first_mut()
	}

	/// Get a reference to a specific [`TagType`]
	pub fn tag(&self, tag_type: &TagType) -> Option<&Tag> {
		self.tags.iter().find(|i| i.tag_type() == tag_type)
	}

	/// Get a mutable reference to a specific [`TagType`]
	pub fn tag_mut(&mut self, tag_type: &TagType) -> Option<&mut Tag> {
		self.tags.iter_mut().find(|i| i.tag_type() == tag_type)
	}

	/// Inserts a [`Tag`]
	///
	/// If a tag is replaced, it will be returned
	pub fn insert_tag(&mut self, tag: Tag) -> Option<Tag> {
		let tag_type = *tag.tag_type();

		if self.supports_tag_type(tag_type) {
			let ret = self.remove_tag(tag_type);
			self.tags.push(tag);

			return ret;
		}

		None
	}

	/// Removes a specific [`TagType`]
	///
	/// This will return the tag if it is removed
	pub fn remove_tag(&mut self, tag_type: TagType) -> Option<Tag> {
		self.tags
			.iter()
			.position(|t| t.tag_type() == &tag_type)
			.map(|pos| self.tags.remove(pos))
	}

	/// Removes all tags from the file
	pub fn clear(&mut self) {
		self.tags.clear()
	}

	/// Attempts to write all tags to a path
	///
	/// # Errors
	///
	/// See [`TaggedFile::save_to`]
	pub fn save_to_path(&self, path: impl AsRef<Path>) -> Result<()> {
		self.save_to(&mut OpenOptions::new().read(true).write(true).open(path)?)
	}

	/// Attempts to write all tags to a file
	///
	/// # Errors
	///
	/// See [`Tag::save_to`], however this is applicable to every tag in the `TaggedFile`.
	pub fn save_to(&self, file: &mut File) -> Result<()> {
		for tag in &self.tags {
			tag.save_to(file)?;
		}

		Ok(())
	}
}

impl TaggedFile {
	/// Returns the file's [`FileType`]
	pub fn file_type(&self) -> &FileType {
		&self.ty
	}

	/// Returns a reference to the file's [`FileProperties`]
	pub fn properties(&self) -> &FileProperties {
		&self.properties
	}
}

#[derive(PartialEq, Copy, Clone, Debug)]
#[allow(missing_docs)]
#[non_exhaustive]
/// The type of file read
pub enum FileType {
	AIFF,
	APE,
	FLAC,
	MP3,
	MP4,
	Opus,
	Vorbis,
	WAV,
}

impl FileType {
	#[allow(unreachable_patterns, clippy::match_same_arms)]
	/// Returns the file type's "primary" [`TagType`], or the one most likely to be used in the target format
	///
	/// | [`FileType`]             | [`TagType`]      |
	/// |--------------------------|------------------|
	/// | `AIFF`, `MP3`, `WAV`     | `Id3v2`          |
	/// | `APE`                    | `Ape`            |
	/// | `FLAC`, `Opus`, `Vorbis` | `VorbisComments` |
	/// | `MP4`                    | `Mp4Ilst`        |
	pub fn primary_tag_type(&self) -> TagType {
		match self {
			#[cfg(all(not(feature = "id3v2"), feature = "aiff_text_chunks"))]
			FileType::AIFF => TagType::AiffText,
			#[cfg(all(not(feature = "id3v2"), feature = "riff_info_list"))]
			FileType::WAV => TagType::RiffInfo,
			#[cfg(all(not(feature = "id3v2"), feature = "id3v1"))]
			FileType::MP3 => TagType::Id3v1,
			#[cfg(all(not(feature = "id3v2"), not(feature = "id3v1"), feature = "ape"))]
			FileType::MP3 => TagType::Ape,
			FileType::AIFF | FileType::MP3 | FileType::WAV => TagType::Id3v2,
			#[cfg(all(not(feature = "ape"), feature = "id3v1"))]
			FileType::MP3 => TagType::Id3v1,
			FileType::APE => TagType::Ape,
			FileType::FLAC | FileType::Opus | FileType::Vorbis => TagType::VorbisComments,
			FileType::MP4 => TagType::Mp4Ilst,
		}
	}

	/// Returns if the target `FileType` supports a [`TagType`]
	pub fn supports_tag_type(&self, tag_type: &TagType) -> bool {
		match self {
			#[cfg(feature = "id3v2")]
			FileType::AIFF | FileType::APE | FileType::MP3 | FileType::WAV
				if tag_type == &TagType::Id3v2 =>
			{
				true
			},
			#[cfg(feature = "aiff_text_chunks")]
			FileType::AIFF if tag_type == &TagType::AiffText => true,
			#[cfg(feature = "id3v1")]
			FileType::APE | FileType::MP3 if tag_type == &TagType::Id3v1 => true,
			#[cfg(feature = "ape")]
			FileType::APE | FileType::MP3 if tag_type == &TagType::Ape => true,
			#[cfg(feature = "vorbis_comments")]
			FileType::Opus | FileType::FLAC | FileType::Vorbis => tag_type == &TagType::VorbisComments,
			#[cfg(feature = "mp4_ilst")]
			FileType::MP4 => tag_type == &TagType::Mp4Ilst,
			#[cfg(feature = "riff_info_list")]
			FileType::WAV => tag_type == &TagType::RiffInfo,
			_ => false,
		}
	}

	/// Attempts to extract a [`FileType`] from an extension
	pub fn from_ext<E>(ext: E) -> Option<Self>
	where
		E: AsRef<OsStr>,
	{
		let ext = ext.as_ref().to_str()?.to_ascii_lowercase();

		match ext.as_str() {
			"ape" => Some(Self::APE),
			"aiff" | "aif" => Some(Self::AIFF),
			"mp3" => Some(Self::MP3),
			"wav" | "wave" => Some(Self::WAV),
			"opus" => Some(Self::Opus),
			"flac" => Some(Self::FLAC),
			"ogg" => Some(Self::Vorbis),
			"mp4" | "m4a" | "m4b" | "m4p" | "m4r" | "m4v" | "3gp" => Some(Self::MP4),
			_ => None,
		}
	}

	/// Attempts to extract a [`FileType`] from a path
	///
	/// # Errors
	///
	/// This will return [`ErrorKind::BadExtension`] if the extension didn't map to a `FileType`
	pub fn from_path<P>(path: P) -> Result<Self>
	where
		P: AsRef<Path>,
	{
		let ext = path.as_ref().extension();

		ext.and_then(Self::from_ext).map_or_else(
			|| {
				let ext_err = match ext {
					Some(ext) => ext.to_string_lossy().into_owned(),
					None => String::new(),
				};

				Err(LoftyError::new(ErrorKind::BadExtension(ext_err)))
			},
			Ok,
		)
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

		// Safe to unwrap, since we return early on an empty buffer
		match buf.first().unwrap() {
			77 if buf.starts_with(b"MAC") => Some(Self::APE),
			255 if buf.len() >= 2 && verify_frame_sync([buf[0], buf[1]]) => Some(Self::MP3),
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
			_ if buf.len() >= 8 && &buf[4..8] == b"ftyp" => Some(Self::MP4),
			_ => None,
		}
	}
}
