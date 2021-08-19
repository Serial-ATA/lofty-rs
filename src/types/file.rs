use crate::logic::id3::v2::Id3v2Version;
use crate::{FileProperties, LoftyError, Result, Tag, TagType};

use byteorder::ReadBytesExt;
use std::convert::TryInto;
use std::io::{Read, Seek, SeekFrom};

pub trait AudioFile {
	fn read_from<R>(reader: &mut R) -> Result<Self>
	where
		R: Read + Seek,
		Self: Sized;
	fn properties(&self) -> &FileProperties;
	fn contains_tag(&self) -> bool;
	fn contains_tag_type(&self, tag_type: &TagType) -> bool;
}

// TODO: store vendor string
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
		let pred = match self.ty {
			FileType::AIFF | FileType::MP3 | FileType::WAV => {
				|t: &&Tag| t.tag_type() == &TagType::Id3v2(Id3v2Version::V4)
			},
			FileType::APE => |t: &&Tag| t.tag_type() == &TagType::Ape,
			FileType::FLAC | FileType::Opus | FileType::Vorbis => {
				|t: &&Tag| t.tag_type() == &TagType::VorbisComments
			},
			FileType::MP4 => |t: &&Tag| t.tag_type() == &TagType::Mp4Atom,
		};

		self.tags.iter().find(pred)
	}

	/// Gets a mutable reference to the file's "Primary tag"
	///
	/// See [`primary_tag`](Self::primary_tag) for an explanation
	pub fn primary_tag_mut(&mut self) -> Option<&mut Tag> {
		let pred = match self.ty {
			FileType::AIFF | FileType::MP3 | FileType::WAV => {
				|t: &&mut Tag| t.tag_type() == &TagType::Id3v2(Id3v2Version::V4)
			},
			FileType::APE => |t: &&mut Tag| t.tag_type() == &TagType::Ape,
			FileType::FLAC | FileType::Opus | FileType::Vorbis => {
				|t: &&mut Tag| t.tag_type() == &TagType::VorbisComments
			},
			FileType::MP4 => |t: &&mut Tag| t.tag_type() == &TagType::Mp4Atom,
		};

		self.tags.iter_mut().find(pred)
	}

	/// Gets the first tag, if there are any
	pub fn first_tag(&self) -> Option<&Tag> {
		self.tags.first()
	}

	/// Gets a mutable reference to the first tag, if there are any
	pub fn first_tag_mut(&mut self) -> Option<&mut Tag> {
		self.tags.first_mut()
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

/// The type of file read
#[allow(missing_docs)]
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
	pub(crate) fn try_from_ext(ext: &str) -> Result<Self> {
		match ext {
			"ape" => Ok(Self::APE),
			"aiff" | "aif" => Ok(Self::AIFF),
			"mp3" => Ok(Self::MP3),
			"wav" | "wave" => Ok(Self::WAV),
			"opus" => Ok(Self::Opus),
			"flac" => Ok(Self::FLAC),
			"ogg" => Ok(Self::Vorbis),
			"m4a" | "m4b" | "m4p" | "m4v" | "isom" | "mp4" => Ok(Self::MP4),
			"oga" => Err(LoftyError::Ogg(
				"Files with extension \"oga\" must have their type determined by content",
			)),
			_ => Err(LoftyError::UnsupportedFormat(ext.to_string())),
		}
	}

	// TODO
	pub(crate) fn try_from_sig<R>(data: &mut R) -> Result<Self>
	where
		R: Read + Seek,
	{
		use crate::logic::{id3::decode_u32, mpeg::header::verify_frame_sync};

		if data.seek(SeekFrom::End(0))? == 0 {
			return Err(LoftyError::EmptyFile);
		}

		data.seek(SeekFrom::Start(0))?;

		let mut sig = vec![0; 10];
		data.read_exact(&mut sig)?;

		data.seek(SeekFrom::Start(0))?;

		match sig.first().unwrap() {
			77 if sig.starts_with(b"MAC") => Ok(Self::APE),
			_ if verify_frame_sync(sig[0], sig[1])
				|| ((sig.starts_with(b"ID3") || sig.starts_with(b"id3")) && {
					let size = decode_u32(u32::from_be_bytes(
						sig[6..10]
							.try_into()
							.map_err(|_| LoftyError::UnknownFormat)?,
					));

					data.seek(SeekFrom::Start(u64::from(10 + size)))?;

					let b1 = data.read_u8()?;
					let b2 = data.read_u8()?;

					data.seek(SeekFrom::Start(0))?;

					verify_frame_sync(b1, b2)
				}) =>
			{
				Ok(Self::MP3)
			},
			70 if sig.starts_with(b"FORM") => {
				data.seek(SeekFrom::Start(8))?;

				let mut id = [0; 4];
				data.read_exact(&mut id)?;

				data.seek(SeekFrom::Start(0))?;

				if &id == b"AIFF" || &id == b"AIFC" {
					return Ok(Self::AIFF);
				}

				Err(LoftyError::UnknownFormat)
			},
			102 if sig.starts_with(b"fLaC") => Ok(Self::FLAC),
			79 if sig.starts_with(b"OggS") => {
				data.seek(SeekFrom::Start(28))?;

				let mut ident_sig = vec![0; 8];
				data.read_exact(&mut ident_sig)?;

				data.seek(SeekFrom::Start(0))?;

				if &ident_sig[1..7] == b"vorbis" {
					return Ok(Self::Vorbis);
				}

				if &ident_sig[..] == b"OpusHead" {
					return Ok(Self::Opus);
				}

				Err(LoftyError::UnknownFormat)
			},
			82 if sig.starts_with(b"RIFF") => {
				data.seek(SeekFrom::Start(8))?;

				let mut id = [0; 4];
				data.read_exact(&mut id)?;

				data.seek(SeekFrom::Start(0))?;

				if &id == b"WAVE" {
					return Ok(Self::WAV);
				}

				Err(LoftyError::UnknownFormat)
			},
			_ if &sig[4..8] == b"ftyp" => Ok(Self::MP4),
			_ => Err(LoftyError::UnknownFormat),
		}
	}
}
