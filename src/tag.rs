#[allow(clippy::wildcard_imports)]
use super::{components::tags::*, AudioTag, Error, Result};
use std::path::Path;

#[cfg(feature = "ape")]
const MAC: [u8; 3] = [77, 65, 67];
#[cfg(feature = "mp3")]
const ID3: [u8; 3] = [73, 68, 51];
#[cfg(feature = "mp4")]
const FTYP: [u8; 4] = [102, 116, 121, 112];
#[cfg(feature = "vorbis")]
const OPUSHEAD: [u8; 8] = [79, 112, 117, 115, 72, 101, 97, 100];
#[cfg(feature = "vorbis")]
const FLAC: [u8; 4] = [102, 76, 97, 67];
#[cfg(feature = "vorbis")]
const OGGS: [u8; 4] = [79, 103, 103, 83];
#[cfg(feature = "vorbis")]
const VORBIS: [u8; 6] = [118, 111, 114, 98, 105, 115];
#[cfg(feature = "wav")]
const RIFF: [u8; 4] = [82, 73, 70, 70];

/// A builder for `Box<dyn AudioTag>`. If you do not want a trait object, you can use individual types.
#[derive(Default)]
pub struct Tag(Option<TagType>);

/// Used in Tag::read_from_path to choose the method to determine the tag type
pub enum DetermineFrom {
	/// Determine the format from the file extension
	Extension,
	/// Determine the format by reading the file, and matching the signature
	Signature,
}

impl Tag {
	/// Initiate a new Tag
	pub fn new() -> Self {
		Self::default()
	}

	/// This function can be used to specify a `TagType`, so there's no guessing
	#[allow(clippy::unused_self)]
	pub fn with_tag_type(self, tag_type: TagType) -> Self {
		Self(Some(tag_type))
	}

	/// Path of the file to read, and the method to determine the tag type
	///
	/// # Errors
	///
	/// * `path` either has no extension, or the extension is not valid unicode (DetermineFrom::Extension)
	/// * `path` has an unsupported/unknown extension (DetermineFrom::Extension)
	/// * `path` does not exist (DetermineFrom::Signature)
	///
	/// # Warning
	/// Using DetermineFrom::Extension on a `wav`/`wave` file will **always** assume there's an ID3 tag.
	/// DetermineFrom::Signature is recommended instead, in the event that a RIFF INFO list is present instead.
	/// However, if both are present, only the ID3 tag is read.
	pub fn read_from_path(
		&self,
		path: impl AsRef<Path>,
		method: DetermineFrom,
	) -> Result<Box<dyn AudioTag>> {
		let tag_type = match method {
			DetermineFrom::Extension => {
				let extension = path
					.as_ref()
					.extension()
					.ok_or(Error::UnknownFileExtension)?;
				let extension_str = extension.to_str().ok_or(Error::UnknownFileExtension)?;

				TagType::try_from_ext(extension_str)?
			},
			DetermineFrom::Signature => TagType::try_from_sig(&std::fs::read(path.as_ref())?)?,
		};

		match tag_type {
			#[cfg(feature = "ape")]
			TagType::Ape => Ok(Box::new(ApeTag::read_from_path(path)?)),
			#[cfg(feature = "mp3")]
			TagType::Id3v2 | TagType::Riff(RiffFormat::ID3) => {
				Ok(Box::new(Id3v2Tag::read_from_path(path, tag_type)?))
			},
			#[cfg(feature = "mp4")]
			TagType::Mp4 => Ok(Box::new(Mp4Tag::read_from_path(path)?)),
			#[cfg(feature = "wav")]
			TagType::Riff(RiffFormat::Info) => Ok(Box::new(RiffTag::read_from_path(path)?)),
			#[cfg(feature = "vorbis")]
			TagType::Vorbis(format) => Ok(Box::new(VorbisTag::read_from_path(path, format.clone())?)),
		}
	}
}

/// The tag type, based on the file extension.
#[derive(Clone, Debug, PartialEq)]
pub enum TagType {
	#[cfg(feature = "ape")]
	/// Common file extensions: `.ape`
	Ape,
	#[cfg(feature = "mp3")]
	/// Common file extensions: `.mp3`
	Id3v2,
	#[cfg(feature = "mp4")]
	/// Common file extensions: `.mp4, .m4a, .m4p, .m4b, .m4r, .m4v`
	Mp4,
	#[cfg(feature = "vorbis")]
	/// Represents multiple formats, see [`VorbisFormat`] for extensions.
	Vorbis(VorbisFormat),
	#[cfg(feature = "wav")]
	/// Represents multiple formats, see [`RiffFormat`] for extensions.
	Riff(RiffFormat),
}

#[derive(Clone, Debug, PartialEq)]
#[cfg(feature = "vorbis")]
pub enum VorbisFormat {
	#[cfg(feature = "vorbis")]
	/// Common file extensions:  `.ogg, .oga`
	Ogg,
	#[cfg(feature = "vorbis")]
	/// Common file extensions: `.opus`
	Opus,
	#[cfg(feature = "vorbis")]
	/// Common file extensions: `.flac`
	Flac,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg(feature = "wav")]
pub enum RiffFormat {
	#[cfg(feature = "wav")]
	/// Metadata is stored in a RIFF INFO list
	Info,
	#[cfg(feature = "mp3")]
	/// Metadata is stored in an ID3 tag
	ID3,
}

impl TagType {
	fn try_from_ext(ext: &str) -> Result<Self> {
		match ext {
			#[cfg(feature = "ape")]
			"ape" => Ok(Self::Ape),
			#[cfg(feature = "mp3")]
			"mp3" => Ok(Self::Id3v2),
			#[cfg(feature = "vorbis")]
			"opus" => Ok(Self::Vorbis(VorbisFormat::Opus)),
			#[cfg(feature = "vorbis")]
			"flac" => Ok(Self::Vorbis(VorbisFormat::Flac)),
			#[cfg(feature = "vorbis")]
			"ogg" | "oga" => Ok(Self::Vorbis(VorbisFormat::Ogg)),
			#[cfg(feature = "mp4")]
			"m4a" | "m4b" | "m4p" | "m4v" | "isom" | "mp4" => Ok(Self::Mp4),
			#[cfg(feature = "wav")]
			"wav" | "wave" => Ok(Self::Id3v2),
			_ => Err(Error::UnsupportedFormat(ext.to_owned())),
		}
	}
	fn try_from_sig(data: &[u8]) -> Result<Self> {
		if data.len() < 1 {
			return Err(Error::EmptyFile);
		}

		match data[0] {
			#[cfg(feature = "ape")]
			77 if data.starts_with(&MAC) => Ok(Self::Ape),
			#[cfg(feature = "mp3")]
			73 if data.starts_with(&ID3) => Ok(Self::Id3v2),
			#[cfg(feature = "vorbis")]
			102 if data.starts_with(&FLAC) => Ok(Self::Vorbis(VorbisFormat::Flac)),
			#[cfg(feature = "vorbis")]
			79 if data.starts_with(&OGGS) => {
				if data[29..35] == VORBIS {
					return Ok(Self::Vorbis(VorbisFormat::Ogg));
				}

				if data[28..36] == OPUSHEAD {
					return Ok(Self::Vorbis(VorbisFormat::Opus));
				}

				Err(Error::UnknownFormat)
			},
			#[cfg(feature = "wav")]
			82 if data.starts_with(&RIFF) => {
				#[cfg(feature = "mp3")]
				{
					use byteorder::{LittleEndian, ReadBytesExt};
					use std::io::Cursor;

					let mut data = Cursor::new(&data[12..]);

					let mut reading = true;
					let mut found_id3 = false;

					while reading {
						if let (Ok(fourcc), Ok(size)) = (
							data.read_u32::<LittleEndian>(),
							data.read_u32::<LittleEndian>(),
						) {
							if &fourcc.to_le_bytes() == b"ID3 " {
								found_id3 = true;
								break;
							}

							data.set_position(data.position() + size as u64)
						} else {
							reading = false
						}
					}

					if found_id3 {
						return Ok(Self::Riff(RiffFormat::ID3));
					}
				}

				Ok(Self::Riff(RiffFormat::Info))
			},
			#[cfg(feature = "mp4")]
			_ if data[4..8] == FTYP => Ok(Self::Mp4),
			_ => Err(Error::UnknownFormat),
		}
	}
}
