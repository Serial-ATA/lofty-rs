mod constants;
mod properties;
pub(crate) mod read;
mod tag;
pub(crate) mod write;

use crate::{FileProperties, Tag, Result};

use std::io::{Read, Seek};

#[allow(dead_code)]
pub struct ApeFile {
	pub id3v1: Option<Tag>,
	pub id3v2: Option<Tag>,
	pub ape: Option<Tag>,
	pub properties: FileProperties,
}

impl ApeFile {
	pub(crate) fn read_from<R>(reader: &mut R) -> Result<Self>
		where
			R: Read + Seek,
	{
		self::read::read_from(reader)
	}
}