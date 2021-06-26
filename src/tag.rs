#[allow(clippy::wildcard_imports)]
use crate::components::tags::*;
use crate::{AudioTag, LoftyError, Result};
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::Path;

#[cfg(feature = "format-ape")]
const MAC: [u8; 3] = [77, 65, 67];
#[cfg(feature = "format-id3")]
const ID3: [u8; 3] = [73, 68, 51];
#[cfg(feature = "format-id3")]
const FORM: [u8; 4] = [70, 79, 82, 77];
#[cfg(feature = "format-mp4")]
const FTYP: [u8; 4] = [102, 116, 121, 112];
#[cfg(feature = "format-opus")]
const OPUSHEAD: [u8; 8] = [79, 112, 117, 115, 72, 101, 97, 100];
#[cfg(feature = "format-flac")]
const FLAC: [u8; 4] = [102, 76, 97, 67];
#[cfg(any(
	feature = "format-vorbis",
	feature = "format-opus",
	feature = "format-flac"
))]
const OGGS: [u8; 4] = [79, 103, 103, 83];
#[cfg(feature = "format-vorbis")]
const VORBIS: [u8; 6] = [118, 111, 114, 98, 105, 115];
#[cfg(feature = "format-riff")]
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
	/// * `path` does not exist
	/// * `path` either has no extension, or the extension is not valid UTF-8
	/// * `path` has an unsupported/unknown extension
	///
	/// # Warning
	/// Using this on a `wav`/`wave`/`riff` file will **always** assume there's an ID3 tag.
	/// [`read_from_path_signature`](Tag::read_from_path_signature) is recommended, in the event that a RIFF INFO list is present instead.
	pub fn read_from_path(&self, path: impl AsRef<Path>) -> Result<Box<dyn AudioTag>> {
		let mut c = Cursor::new(std::fs::read(&path)?);

		let tag_type = self.0.clone().unwrap_or({
			let extension = path
				.as_ref()
				.extension()
				.ok_or(LoftyError::UnknownFileExtension)?;

			let extension_str = extension.to_str().ok_or(LoftyError::UnknownFileExtension)?;

			TagType::try_from_ext(extension_str)?
		});

		Self::match_tag(&mut c, tag_type)
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
		let mut c = Cursor::new(std::fs::read(&path)?);

		let tag_type = self.0.clone().unwrap_or(TagType::try_from_sig(&mut c)?);

		Self::match_tag(&mut c, tag_type)
	}

	/// Attempts to get the tag format based on the data in the reader
	///
	/// See [`read_from_path_signature`] for important notes, errors, and warnings.
	///
	/// # Errors
	///
	/// Same as [`read_from_path_signature`]
	pub fn read_from_reader<R>(&self, reader: &mut R) -> Result<Box<dyn AudioTag>>
	where
		R: Read + Seek,
	{
		let tag_type = self.0.clone().unwrap_or(TagType::try_from_sig(reader)?);

		Self::match_tag(reader, tag_type)
	}

	fn match_tag<R>(reader: &mut R, tag_type: TagType) -> Result<Box<dyn AudioTag>>
	where
		R: Read + Seek,
	{
		match tag_type {
			#[cfg(feature = "format-ape")]
			TagType::Ape => Ok(Box::new(ApeTag::read_from(reader)?)),
			#[cfg(feature = "format-id3")]
			TagType::Id3v2(format) => Ok(Box::new(Id3v2Tag::read_from(reader, &format)?)),
			#[cfg(feature = "format-mp4")]
			TagType::Mp4 => Ok(Box::new(Mp4Tag::read_from(reader)?)),
			#[cfg(feature = "format-riff")]
			TagType::RiffInfo => Ok(Box::new(RiffTag::read_from(reader)?)),
			#[cfg(any(
				feature = "format-vorbis",
				feature = "format-flac",
				feature = "format-opus"
			))]
			TagType::Ogg(format) => Ok(Box::new(VorbisTag::read_from(reader, &format)?)),
		}
	}
}

/// The tag type, based on the file extension.
#[derive(Clone, Debug, PartialEq)]
pub enum TagType {
	#[cfg(feature = "format-ape")]
	/// Common file extensions: `.ape`
	Ape,
	#[cfg(feature = "format-id3")]
	/// Represents multiple formats, see [`Id3Format`](Id3Format) for extensions.
	Id3v2(Id3Format),
	#[cfg(feature = "format-mp4")]
	/// Common file extensions: `.mp4, .m4a, .m4p, .m4b, .m4r, .m4v`
	Mp4,
	#[cfg(any(
		feature = "format-vorbis",
		feature = "format-opus",
		feature = "format-flac"
	))]
	/// Represents multiple formats, see [`OggFormat`](OggFormat) for extensions.
	Ogg(OggFormat),
	#[cfg(feature = "format-riff")]
	/// Metadata stored in a RIFF INFO chunk
	/// Common file extensions: `.wav, .wave, .riff`
	RiffInfo,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg(any(
	feature = "format-vorbis",
	feature = "format-opus",
	feature = "format-flac"
))]
/// File formats using vorbis comments
pub enum OggFormat {
	#[cfg(feature = "format-vorbis")]
	/// Common file extensions:  `.ogg, .oga`
	Vorbis,
	#[cfg(feature = "format-opus")]
	/// Common file extensions: `.opus`
	Opus,
	#[cfg(feature = "format-flac")]
	/// Common file extensions: `.flac`
	Flac,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg(feature = "format-id3")]
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
			#[cfg(feature = "format-ape")]
			"ape" => Ok(Self::Ape),
			#[cfg(feature = "format-id3")]
			"aiff" | "aif" => Ok(Self::Id3v2(Id3Format::Form)),
			#[cfg(feature = "format-id3")]
			"mp3" => Ok(Self::Id3v2(Id3Format::Default)),
			#[cfg(all(feature = "format-riff", feature = "format-id3"))]
			"wav" | "wave" | "riff" => Ok(Self::Id3v2(Id3Format::Riff)),
			#[cfg(feature = "format-opus")]
			"opus" => Ok(Self::Ogg(OggFormat::Opus)),
			#[cfg(feature = "format-flac")]
			"flac" => Ok(Self::Ogg(OggFormat::Flac)),
			#[cfg(feature = "format-vorbis")]
			"ogg" | "oga" => Ok(Self::Ogg(OggFormat::Vorbis)),
			#[cfg(feature = "format-mp4")]
			"m4a" | "m4b" | "m4p" | "m4v" | "isom" | "mp4" => Ok(Self::Mp4),
			_ => Err(LoftyError::UnsupportedFormat(ext.to_string())),
		}
	}

	fn try_from_sig<R>(data: &mut R) -> Result<Self>
	where
		R: Read + Seek,
	{
		if data.seek(SeekFrom::End(0))? == 0 {
			return Err(LoftyError::EmptyFile);
		}

		data.seek(SeekFrom::Start(0))?;

		let mut sig = vec![0; 8];
		data.read_exact(&mut sig)?;

		data.seek(SeekFrom::Start(0))?;

		match sig.first().unwrap() {
			#[cfg(feature = "format-ape")]
			77 if sig.starts_with(&MAC) => Ok(Self::Ape),
			#[cfg(feature = "format-id3")]
			73 if sig.starts_with(&ID3) => Ok(Self::Id3v2(Id3Format::Default)),
			#[cfg(feature = "format-id3")]
			70 if sig.starts_with(&FORM) => {
				use byteorder::{BigEndian, LittleEndian, ReadBytesExt};

				let mut found_id3 = false;

				while let (Ok(fourcc), Ok(size)) = (
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

					data.seek(SeekFrom::Current(i64::from(u32::from_be_bytes(
						size.to_be_bytes(),
					))))?;
				}

				data.seek(SeekFrom::Start(0))?;

				if found_id3 {
					return Ok(Self::Id3v2(Id3Format::Form));
				}

				// TODO: support AIFF chunks?
				Err(LoftyError::UnknownFormat)
			},
			#[cfg(feature = "format-flac")]
			102 if sig.starts_with(&FLAC) => Ok(Self::Ogg(OggFormat::Flac)),
			#[cfg(any(feature = "format-vorbis", feature = "format-opus"))]
			79 if sig.starts_with(&OGGS) => {
				data.seek(SeekFrom::Start(28))?;

				let mut ident_sig = vec![0; 8];
				data.read_exact(&mut ident_sig)?;

				data.seek(SeekFrom::Start(0))?;

				if ident_sig[1..7] == VORBIS {
					return Ok(Self::Ogg(OggFormat::Vorbis));
				}

				if ident_sig[..] == OPUSHEAD {
					return Ok(Self::Ogg(OggFormat::Opus));
				}

				Err(LoftyError::UnknownFormat)
			},
			#[cfg(feature = "format-riff")]
			82 if sig.starts_with(&RIFF) => {
				#[cfg(feature = "format-id3")]
				{
					use byteorder::{LittleEndian, ReadBytesExt};

					data.seek(SeekFrom::Start(12))?;

					let mut found_id3 = false;

					while let (Ok(fourcc), Ok(size)) = (
						data.read_u32::<LittleEndian>(),
						data.read_u32::<LittleEndian>(),
					) {
						if &fourcc.to_le_bytes() == b"ID3 " {
							found_id3 = true;
							break;
						}

						data.seek(SeekFrom::Current(i64::from(size)))?;
					}

					data.seek(SeekFrom::Start(0))?;

					if found_id3 {
						return Ok(Self::Id3v2(Id3Format::Riff));
					}
				}

				Ok(Self::RiffInfo)
			},
			#[cfg(feature = "format-mp4")]
			_ if sig[4..8] == FTYP => Ok(Self::Mp4),
			_ => Err(LoftyError::UnknownFormat),
		}
	}
}
