use crate::{Error, Result};
use id3::frame::PictureType as id3PicType;
use std::convert::TryFrom;

/// Mime types for covers.
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

/// The picture type
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PictureType {
	CoverFront,
	CoverBack,
	Other,
}

impl From<&id3PicType> for PictureType {
	fn from(inp: &id3PicType) -> Self {
		match inp {
			id3PicType::CoverFront => PictureType::CoverFront,
			id3PicType::CoverBack => PictureType::CoverBack,
			_ => PictureType::Other,
		}
	}
}

/// Represents a picture, with its data and mime type.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Picture {
	pub pic_type: PictureType,
	pub data: Vec<u8>,
	pub mime_type: MimeType,
}

impl Picture {
	pub fn new(pic_type: PictureType, data: Vec<u8>, mime_type: MimeType) -> Self {
		Self {
			pic_type,
			data,
			mime_type,
		}
	}
}
