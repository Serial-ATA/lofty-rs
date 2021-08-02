mod constants;
pub(crate) mod header;
pub(crate) mod read;

use crate::{FileProperties, Tag, Result};

use std::io::{Read, Seek};

#[allow(dead_code)]
pub struct MpegFile {
	pub id3: Option<Tag>,
	pub ape: Option<Tag>,
	pub properties: FileProperties,
}

impl MpegFile {
	pub(crate) fn read_from<R>(reader: &mut R) -> Result<Self>
		where
			R: Read + Seek,
	{
		self::read::read_from(reader)
	}
}