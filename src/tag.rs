use super::{AudioTag, Error, FlacTag, Id3v2Tag, Mp4Tag, OpusTag, Result, OggTag};
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

		match self.0.unwrap_or(TagType::try_from_ext(extension)?) {
			TagType::Id3v2 => Ok(Box::new(Id3v2Tag::read_from_path(path)?)),
			TagType::Ogg => Ok(Box::new(OggTag::read_from_path(path)?)),
			TagType::Opus => Ok(Box::new(OpusTag::read_from_path(path)?)),
			TagType::Flac => Ok(Box::new(FlacTag::read_from_path(path)?)),
			TagType::Mp4 => Ok(Box::new(Mp4Tag::read_from_path(path)?)),
		}
	}
}

/// The tag type, based on the file extension.
#[derive(Clone, Copy, Debug)]
pub enum TagType {
	/// Common file extensions: `.mp3`
	Id3v2,
	/// Common file extensions:  `.ogg, .oga`
	Ogg,
	/// Common file extensions: `.opus`
	Opus,
	/// Common file extensions: `.flac`
	Flac,
	/// Common file extensions: `.mp4, .m4a, .m4p, .m4b, .m4r, .m4v`
	Mp4,
}

impl TagType {
	fn try_from_ext(ext: &str) -> Result<Self> {
		match ext {
			"mp3" => Ok(Self::Id3v2),
			"opus" => Ok(Self::Opus),
			"flac" => Ok(Self::Flac),
			"ogg" | "oga" => Ok(Self::Ogg),
			"m4a" | "m4b" | "m4p" | "m4v" | "isom" | "mp4" => Ok(Self::Mp4),
			_ => Err(Error::UnsupportedFormat(ext.to_owned())),
		}
	}
}
