use crate::{Error, Result};
use std::convert::TryFrom;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MimeType {
	Png,
	Jpeg,
	Tiff,
	Bmp,
	Gif,
}

impl TryFrom<&str> for MimeType {
	type Error = Error;
	fn try_from(inp: &str) -> Result<Self> {
		Ok(match inp {
			"image/jpeg" => MimeType::Jpeg,
			"image/png" => MimeType::Png,
			"image/tiff" => MimeType::Tiff,
			"image/bmp" => MimeType::Bmp,
			"image/gif" => MimeType::Gif,
			_ => return Err(Error::UnsupportedMimeType(inp.to_owned())),
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
