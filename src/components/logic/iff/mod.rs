use crate::{FileProperties, Tag, Result};

use std::io::{Read, Seek};

pub(crate) mod aiff;
pub(crate) mod riff;

pub struct AiffFile {
	pub properties: FileProperties,
	#[cfg(feature = "format-aiff")]
	pub text_chunks: Option<Tag>,
	#[cfg(feature = "format-id3")]
	pub id3: Option<Tag>,
}

impl AiffFile {
	pub(crate) fn read_from<R>(reader: &mut R) -> Result<Self>
		where
			R: Read + Seek,
	{
		self::aiff::read_from(reader)
	}
}

pub struct WavFile {
	pub properties: FileProperties,
	#[cfg(feature = "format-riff")]
	pub riff_info: Option<Tag>,
	#[cfg(feature = "format-id3")]
	pub id3: Option<Tag>,
}

impl WavFile {
	pub(crate) fn read_from<R>(reader: &mut R) -> Result<Self>
		where
			R: Read + Seek,
	{
		self::riff::read_from(reader)
	}
}