use super::{components::*, AudioTag, Error, Result};
use crate::vorbis_tag::VorbisTag;
use std::path::Path;

/// A builder for `Box<dyn AudioTag>`. If you do not want a trait object, you can use individual types.
#[derive(Default)]
pub struct Tag(Option<TagType>);

impl Tag {
	/// Initiate a new Tag
	pub fn new() -> Self {
		Self::default()
	}
	/// This function can be used to specify a `TagType`, so there's no guessing
	pub fn with_tag_type(self, tag_type: TagType) -> Self {
		Self(Some(tag_type))
	}
	/// Path of the file to read
	pub fn read_from_path(&self, path: impl AsRef<Path>) -> Result<Box<dyn AudioTag>> {
		let extension = path.as_ref().extension().unwrap().to_str().unwrap();

		match self
			.0
			.as_ref()
			.unwrap_or(&TagType::try_from_ext(extension)?)
		{
			#[cfg(feature = "mp3")]
			TagType::Id3v2 => Ok(Box::new(Id3v2Tag::read_from_path(path, None)?)),
			#[cfg(feature = "mp4")]
			TagType::Mp4 => Ok(Box::new(Mp4Tag::read_from_path(path, None)?)),
			#[cfg(feature = "vorbis")]
			id @ _ => Ok(Box::new(VorbisTag::read_from_path(
				path,
				Some(id.to_owned()),
			)?)),
		}
	}
}

/// The tag type, based on the file extension.
#[derive(Clone, Debug, PartialEq)]
pub enum TagType {
	#[cfg(feature = "mp3")]
	/// Common file extensions: `.mp3`
	Id3v2,
	#[cfg(feature = "vorbis")]
	/// Common file extensions:  `.ogg, .oga`
	Ogg,
	#[cfg(feature = "vorbis")]
	/// Common file extensions: `.opus`
	Opus,
	#[cfg(feature = "vorbis")]
	/// Common file extensions: `.flac`
	Flac,
	#[cfg(feature = "mp4")]
	/// Common file extensions: `.mp4, .m4a, .m4p, .m4b, .m4r, .m4v`
	Mp4,
}

impl TagType {
	fn try_from_ext(ext: &str) -> Result<Self> {
		match ext {
			#[cfg(feature = "mp3")]
			"mp3" => Ok(Self::Id3v2),
			#[cfg(feature = "vorbis")]
			"opus" => Ok(Self::Opus),
			#[cfg(feature = "vorbis")]
			"flac" => Ok(Self::Flac),
			#[cfg(feature = "vorbis")]
			"ogg" | "oga" => Ok(Self::Ogg),
			#[cfg(feature = "mp4")]
			"m4a" | "m4b" | "m4p" | "m4v" | "isom" | "mp4" => Ok(Self::Mp4),
			_ => Err(Error::UnsupportedFormat(ext.to_owned())),
		}
	}
}
