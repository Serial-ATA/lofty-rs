#[allow(clippy::wildcard_imports)]
use crate::components::tags::*;
use crate::{AudioTag, Error, Result};
use std::io::Seek;
use std::path::Path;

#[cfg(feature = "ape")]
const MAC: [u8; 3] = [77, 65, 67];
#[cfg(feature = "id3")]
const ID3: [u8; 3] = [73, 68, 51];
#[cfg(feature = "id3")]
const FORM: [u8; 4] = [70, 79, 82, 77];
#[cfg(feature = "mp4")]
const FTYP: [u8; 4] = [102, 116, 121, 112];
#[cfg(feature = "opus")]
const OPUSHEAD: [u8; 8] = [79, 112, 117, 115, 72, 101, 97, 100];
#[cfg(feature = "flac")]
const FLAC: [u8; 4] = [102, 76, 97, 67];
#[cfg(any(feature = "vorbis", feature = "opus", feature = "flac"))]
const OGGS: [u8; 4] = [79, 103, 103, 83];
#[cfg(feature = "vorbis")]
const VORBIS: [u8; 6] = [118, 111, 114, 98, 105, 115];
#[cfg(feature = "riff")]
const RIFF: [u8; 4] = [82, 73, 70, 70];

/// A builder for `Box<dyn AudioTag>`. If you do not want a trait object, you can use individual types.
#[derive(Default)]
pub struct Tag(Option<TagType>);

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

	/// Attempts to get the tag format based on the file extension
	///
	/// NOTE: Since this only looks at the extension, the result could be incorrect.
	///
	///
	/// # Errors
	///
	/// * `path` either has no extension, or the extension is not valid unicode
	/// * `path` has an unsupported/unknown extension
	///
	/// # Warning
	/// Using this on a `wav`/`wave`/`riff` file will **always** assume there's an ID3 tag.
	/// [`read_from_path_signature`](Tag::read_from_path_signature) is recommended, in the event that a RIFF INFO list is present instead.
	pub fn read_from_path(&self, path: impl AsRef<Path>) -> Result<Box<dyn AudioTag>> {
		let tag_type = self.0.clone().unwrap_or({
			let extension = path
				.as_ref()
				.extension()
				.ok_or(Error::UnknownFileExtension)?;
			let extension_str = extension.to_str().ok_or(Error::UnknownFileExtension)?;

			TagType::try_from_ext(extension_str)?
		});

		Self::match_tag(path, tag_type)
	}

	/// Attempts to get the tag format based on the file signature
	///
	/// NOTE: This is *slightly* slower than reading from extension, but more accurate.
	/// The only times were this would really be necessary is if the file format being read
	/// supports more than one metadata format (ex. RIFF), or there is no file extension.
	///
	/// # Errors
	///
	/// * `path` does not exist
	/// * The tag is non-existent/invalid/unknown
	///
	/// # Warning
	/// In the event that a riff file contains both an ID3 tag *and* a RIFF INFO chunk, the ID3 tag will **always** be chosen.
	pub fn read_from_path_signature(&self, path: impl AsRef<Path>) -> Result<Box<dyn AudioTag>> {
		let tag_type = self
			.0
			.clone()
			.unwrap_or(TagType::try_from_sig(&std::fs::read(path.as_ref())?)?);

		Self::match_tag(path, tag_type)
	}

	fn match_tag(path: impl AsRef<Path>, tag_type: TagType) -> Result<Box<dyn AudioTag>> {
		match tag_type {
			#[cfg(feature = "ape")]
			TagType::Ape => Ok(Box::new(ApeTag::read_from_path(path)?)),
			#[cfg(feature = "id3")]
			TagType::Id3v2(underlying) => Ok(Box::new(Id3v2Tag::read_from_path(path, underlying)?)),
			#[cfg(feature = "mp4")]
			TagType::Mp4 => Ok(Box::new(Mp4Tag::read_from_path(path)?)),
			#[cfg(feature = "riff")]
			TagType::RiffInfo => Ok(Box::new(RiffTag::read_from_path(path)?)),
			#[cfg(any(feature = "vorbis", feature = "flac", feature = "opus"))]
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
	#[cfg(feature = "id3")]
	/// Represents multiple formats, see [`Id3Format`](Id3Format) for extensions.
	Id3v2(Id3Format),
	#[cfg(feature = "mp4")]
	/// Common file extensions: `.mp4, .m4a, .m4p, .m4b, .m4r, .m4v`
	Mp4,
	#[cfg(any(feature = "vorbis", feature = "opus", feature = "flac"))]
	/// Represents multiple formats, see [`VorbisFormat`](VorbisFormat) for extensions.
	Vorbis(VorbisFormat),
	#[cfg(feature = "riff")]
	/// Metadata stored in a RIFF INFO chunk
	/// Common file extensions: `.wav, .wave, .riff`
	RiffInfo,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg(any(feature = "vorbis", feature = "opus", feature = "flac"))]
/// File formats using vorbis comments
pub enum VorbisFormat {
	#[cfg(feature = "vorbis")]
	/// Common file extensions:  `.ogg, .oga`
	Ogg,
	#[cfg(feature = "opus")]
	/// Common file extensions: `.opus`
	Opus,
	#[cfg(feature = "flac")]
	/// Common file extensions: `.flac`
	Flac,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg(feature = "id3")]
/// ID3 tag's underlying format
pub enum Id3Format {
	/// MP3
	Default,
	/// AIFF
	Form,
	/// RIFF/WAV/WAVE
	Riff,
}

impl TagType {
	fn try_from_ext(ext: &str) -> Result<Self> {
		match ext {
			#[cfg(feature = "ape")]
			"ape" => Ok(Self::Ape),
			#[cfg(feature = "id3")]
			"aiff" | "aif" => Ok(Self::Id3v2(Id3Format::Form)),
			#[cfg(feature = "id3")]
			"mp3" => Ok(Self::Id3v2(Id3Format::Default)),
			#[cfg(all(feature = "riff", feature = "id3"))]
			"wav" | "wave" | "riff" => Ok(Self::Id3v2(Id3Format::Riff)),
			#[cfg(feature = "opus")]
			"opus" => Ok(Self::Vorbis(VorbisFormat::Opus)),
			#[cfg(feature = "flac")]
			"flac" => Ok(Self::Vorbis(VorbisFormat::Flac)),
			#[cfg(feature = "vorbis")]
			"ogg" | "oga" => Ok(Self::Vorbis(VorbisFormat::Ogg)),
			#[cfg(feature = "mp4")]
			"m4a" | "m4b" | "m4p" | "m4v" | "isom" | "mp4" => Ok(Self::Mp4),
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
			#[cfg(feature = "id3")]
			73 if data.starts_with(&ID3) => Ok(Self::Id3v2(Id3Format::Default)),
			#[cfg(feature = "id3")]
			70 if data.starts_with(&FORM) => {
				use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
				use std::io::{Cursor, SeekFrom};

				let mut data = Cursor::new(data);
				let mut found_id3 = false;

				loop {
					if let (Ok(fourcc), Ok(size)) = (
						data.read_u32::<LittleEndian>(),
						data.read_u32::<BigEndian>(),
					) {
						if fourcc.to_le_bytes() == FORM {
							data.seek(SeekFrom::Current(4))?;
							continue;
						}

						if fourcc.to_le_bytes()[..3] == ID3 {
							found_id3 = true;
							break;
						}

						data.seek(SeekFrom::Current(
							u32::from_be_bytes(size.to_be_bytes()) as i64
						))?;
					} else {
						break;
					}
				}

				if found_id3 {
					return Ok(Self::Id3v2(Id3Format::Form));
				}

				// TODO: support AIFF chunks?
				Err(Error::UnknownFormat)
			},
			#[cfg(feature = "flac")]
			102 if data.starts_with(&FLAC) => Ok(Self::Vorbis(VorbisFormat::Flac)),
			#[cfg(any(feature = "vorbis", feature = "opus"))]
			79 if data.starts_with(&OGGS) => {
				if data[29..35] == VORBIS {
					return Ok(Self::Vorbis(VorbisFormat::Ogg));
				}

				if data[28..36] == OPUSHEAD {
					return Ok(Self::Vorbis(VorbisFormat::Opus));
				}

				Err(Error::UnknownFormat)
			},
			#[cfg(feature = "riff")]
			82 if data.starts_with(&RIFF) => {
				#[cfg(feature = "id3")]
				{
					use byteorder::{LittleEndian, ReadBytesExt};
					use std::io::Cursor;

					let mut data = Cursor::new(&data[12..]);

					let mut found_id3 = false;

					loop {
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
							break;
						}
					}

					if found_id3 {
						return Ok(Self::Id3v2(Id3Format::Riff));
					}
				}

				Ok(Self::RiffInfo)
			},
			#[cfg(feature = "mp4")]
			_ if data[4..8] == FTYP => Ok(Self::Mp4),
			_ => Err(Error::UnknownFormat),
		}
	}
}
