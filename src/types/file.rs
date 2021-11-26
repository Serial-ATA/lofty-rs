use super::properties::FileProperties;
use super::tag::{Tag, TagType};
use crate::error::{LoftyError, Result};

use std::convert::TryInto;
use std::io::{Read, Seek, SeekFrom};

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
	fn read_from<R>(reader: &mut R) -> Result<Self>
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

impl TaggedFile {
	/// Gets the file's "Primary tag", or the one most likely to be used in the target format
	///
	/// | [`FileType`]             | [`TagType`]      |
	/// |--------------------------|------------------|
	/// | `AIFF`, `MP3`, `WAV`     | `Id3v2`          |
	/// | `APE`                    | `Ape`            |
	/// | `FLAC`, `Opus`, `Vorbis` | `VorbisComments` |
	/// | `MP4`                    | `Mp4Atom`        |
	pub fn primary_tag(&self) -> Option<&Tag> {
		self.tag(&Self::primary_tag_type(self.ty))
	}

	/// Gets a mutable reference to the file's "Primary tag"
	///
	/// See [`primary_tag`](Self::primary_tag) for an explanation
	pub fn primary_tag_mut(&mut self) -> Option<&mut Tag> {
		self.tag_mut(&Self::primary_tag_type(self.ty))
	}

	fn primary_tag_type(f_ty: FileType) -> TagType {
		match f_ty {
			#[cfg(feature = "id3v2")]
			FileType::AIFF | FileType::MP3 | FileType::WAV => TagType::Id3v2,
			#[cfg(feature = "ape")]
			FileType::APE => TagType::Ape,
			#[cfg(feature = "vorbis_comments")]
			FileType::FLAC | FileType::Opus | FileType::Vorbis => TagType::VorbisComments,
			#[cfg(feature = "mp4_atoms")]
			FileType::MP4 => TagType::Mp4Atom,
		}
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
	/// Returns if the target FileType supports a [`TagType`]
	pub fn supports_tag_type(&self, tag_type: &TagType) -> bool {
		match self {
			FileType::AIFF => tag_type == &TagType::Id3v2 || tag_type == &TagType::AiffText,
			FileType::APE => {
				tag_type == &TagType::Ape
					|| tag_type == &TagType::Id3v1
					|| tag_type == &TagType::Id3v2
			},
			FileType::MP3 => {
				tag_type == &TagType::Id3v2
					|| tag_type == &TagType::Ape
					|| tag_type == &TagType::Id3v1
			},
			FileType::Opus | FileType::FLAC | FileType::Vorbis => {
				tag_type == &TagType::VorbisComments
			},
			FileType::MP4 => tag_type == &TagType::Mp4Atom,
			FileType::WAV => tag_type == &TagType::Id3v2 || tag_type == &TagType::RiffInfo,
		}
	}

	pub(crate) fn try_from_ext(ext: &str) -> Result<Self> {
		match ext {
			"ape" => Ok(Self::APE),
			"aiff" | "aif" => Ok(Self::AIFF),
			"mp3" => Ok(Self::MP3),
			"wav" | "wave" => Ok(Self::WAV),
			"opus" => Ok(Self::Opus),
			"flac" => Ok(Self::FLAC),
			"ogg" => Ok(Self::Vorbis),
			"mp4" | "m4a" | "m4b" | "m4p" | "m4r" | "m4v" | "3gp" => Ok(Self::MP4),
			"oga" => Err(LoftyError::Ogg(
				"Files with extension \"oga\" must have their type determined by content",
			)),
			_ => Err(LoftyError::BadExtension(ext.to_string())),
		}
	}

	// TODO
	pub(crate) fn try_from_sig<R>(data: &mut R) -> Result<Self>
	where
		R: Read + Seek,
	{
		use crate::logic::{id3::unsynch_u32, mp3::header::verify_frame_sync};

		if data.seek(SeekFrom::End(0))? == 0 {
			return Err(LoftyError::EmptyFile);
		}

		data.seek(SeekFrom::Start(0))?;

		let mut sig = [0; 10];
		data.read_exact(&mut sig)?;

		let ret = match sig.first().unwrap() {
			77 if sig.starts_with(b"MAC") => Ok(Self::APE),
			73 if sig.starts_with(b"ID3") => {
				let size = unsynch_u32(u32::from_be_bytes(
					sig[6..10]
						.try_into()
						.map_err(|_| LoftyError::UnknownFormat)?,
				));

				data.seek(SeekFrom::Start(u64::from(10 + size)))?;

				let mut ident = [0; 3];
				data.read_exact(&mut ident)?;

				if &ident == b"MAC" {
					Ok(Self::APE)
				} else if verify_frame_sync([ident[0], ident[1]]) {
					Ok(Self::MP3)
				} else {
					Err(LoftyError::UnknownFormat)
				}
			},
			_ if verify_frame_sync([sig[0], sig[1]]) => Ok(Self::MP3),
			70 if sig.starts_with(b"FORM") => {
				let mut id_remaining = [0; 2];
				data.read_exact(&mut id_remaining)?;

				let id = &[sig[8], sig[9], id_remaining[0], id_remaining[1]];

				if id == b"AIFF" || id == b"AIFC" {
					Ok(Self::AIFF)
				} else {
					Err(LoftyError::UnknownFormat)
				}
			},
			102 if sig.starts_with(b"fLaC") => Ok(Self::FLAC),
			79 if sig.starts_with(b"OggS") => {
				data.seek(SeekFrom::Start(28))?;

				let mut ident_sig = [0; 8];
				data.read_exact(&mut ident_sig)?;

				if &ident_sig[1..7] == b"vorbis" {
					Ok(Self::Vorbis)
				} else if &ident_sig[..] == b"OpusHead" {
					Ok(Self::Opus)
				} else {
					Err(LoftyError::UnknownFormat)
				}
			},
			82 if sig.starts_with(b"RIFF") => {
				let mut id_remaining = [0; 2];
				data.read_exact(&mut id_remaining)?;

				if &[sig[8], sig[9], id_remaining[0], id_remaining[1]] == b"WAVE" {
					Ok(Self::WAV)
				} else {
					Err(LoftyError::UnknownFormat)
				}
			},
			_ if &sig[4..8] == b"ftyp" => Ok(Self::MP4),
			_ => Err(LoftyError::UnknownFormat),
		};

		data.seek(SeekFrom::Start(0))?;

		ret
	}
}
