pub use super::*;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MimeType {
	Png,
	Jpeg,
	Tiff,
	Bmp,
	Gif,
}

impl TryFrom<&str> for MimeType {
	type Error = TaggedError;
	fn try_from(inp: &str) -> crate::Result<Self> {
		Ok(match inp {
			"image/jpeg" => MimeType::Jpeg,
			"image/png" => MimeType::Png,
			"image/tiff" => MimeType::Tiff,
			"image/bmp" => MimeType::Bmp,
			"image/gif" => MimeType::Gif,
			_ => return Err(TaggedError::UnsupportedMimeType(inp.to_owned())),
		})
	}
}

impl From<MimeType> for &'static str {
	fn from(mt: MimeType) -> Self {
		match mt {
			MimeType::Jpeg => "image/jpeg",
			MimeType::Png => "image/png",
			MimeType::Tiff => "image/tiff",
			MimeType::Bmp => "image/bmp",
			MimeType::Gif => "image/gif",
		}
	}
}

impl From<MimeType> for String {
	fn from(mt: MimeType) -> Self {
		<MimeType as Into<&'static str>>::into(mt).to_owned()
	}
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Picture<'a> {
	pub data: &'a [u8],
	pub mime_type: MimeType,
}

impl<'a> Picture<'a> {
	pub fn new(data: &'a [u8], mime_type: MimeType) -> Self {
		Self { data, mime_type }
	}
}

/// A struct for representing an album for convenience.
#[derive(Debug)]
pub struct Album<'a> {
	pub title: &'a str,
	pub artist: Option<&'a str>,
	pub cover: Option<Picture<'a>>,
}

impl<'a> Album<'a> {
	pub fn with_title(title: &'a str) -> Self {
		Self {
			title,
			artist: None,
			cover: None,
		}
	}
	pub fn and_artist(mut self, artist: &'a str) -> Self {
		self.artist = Some(artist);
		self
	}
	pub fn and_cover(mut self, cover: Picture<'a>) -> Self {
		self.cover = Some(cover);
		self
	}
	pub fn with_all(title: &'a str, artist: &'a str, cover: Picture<'a>) -> Self {
		Self {
			title,
			artist: Some(artist),
			cover: Some(cover),
		}
	}
}
