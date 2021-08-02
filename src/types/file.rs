use crate::components::logic::ape::ApeFile;
use crate::components::logic::iff::{AiffFile, RiffFile};
use crate::components::logic::mpeg::MpegFile;
use crate::{FileProperties, LoftyError, Tag, Result};

use std::io::{Read, Seek, SeekFrom};
use std::convert::TryInto;
use byteorder::ReadBytesExt;

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
			_ if ext == "oga" => Err(LoftyError::Ogg(
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
		#[cfg(feature = "format-id3")]
		use crate::components::logic::{id3::decode_u32, mpeg::header::verify_frame_sync};

		if data.seek(SeekFrom::End(0))? == 0 {
			return Err(LoftyError::EmptyFile);
		}

		data.seek(SeekFrom::Start(0))?;

		let mut sig = vec![0; 10];
		data.read_exact(&mut sig)?;

		data.seek(SeekFrom::Start(0))?;

		match sig.first().unwrap() {
			#[cfg(feature = "format-ape")]
			77 if sig.starts_with(b"MAC") => Ok(Self::APE),
			#[cfg(feature = "format-id3")]
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
			#[cfg(any(feature = "format-id3", feature = "format-aiff"))]
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
			#[cfg(feature = "format-flac")]
			102 if sig.starts_with(b"fLaC") => Ok(Self::FLAC),
			#[cfg(any(feature = "format-vorbis", feature = "format-opus"))]
			79 if sig.starts_with(b"OggS") => {
				data.seek(SeekFrom::Start(28))?;

				let mut ident_sig = vec![0; 8];
				data.read_exact(&mut ident_sig)?;

				data.seek(SeekFrom::Start(0))?;

				#[cfg(feature = "format-vorbis")]
				if &ident_sig[1..7] == b"vorbis" {
					return Ok(Self::Vorbis);
				}

				#[cfg(feature = "format-opus")]
				if &ident_sig[..] == b"OpusHead" {
					return Ok(Self::Opus);
				}

				Err(LoftyError::UnknownFormat)
			},
			#[cfg(feature = "format-riff")]
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
			#[cfg(feature = "format-mp4")]
			_ if &sig[4..8] == b"ftyp" => Ok(Self::MP4),
			_ => Err(LoftyError::UnknownFormat),
		}
	}
}

pub struct TaggedFile {
	pub ty: FileType,
	pub properties: FileProperties,
	pub tags: Vec<Tag>,
}

impl From<MpegFile> for TaggedFile {
	fn from(inp: MpegFile) -> Self {
		// TODO
	}
}

impl From<ApeFile> for TaggedFile {
	fn from(inp: ApeFile) -> Self {
		// TODO
	}
}

impl From<AiffFile> for TaggedFile {
	fn from(inp: AiffFile) -> Self {
		// TODO
	}
}

impl From<RiffFile> for TaggedFile {
	fn from(inp: RiffFile) -> Self {
		// TODO
	}
}
