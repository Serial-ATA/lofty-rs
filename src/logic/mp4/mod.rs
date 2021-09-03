mod atom;
mod ilst;
mod moov;
mod properties;
pub(crate) mod read;
mod trak;

use crate::types::file::AudioFile;
use crate::{FileProperties, Result, Tag, TagType};

use std::io::{Read, Seek};

#[allow(dead_code)]
/// An MP4 file
pub struct Mp4File {
	/// The file format from ftyp's "major brand" (Ex. "M4A ")
	pub(crate) ftyp: String,
	/// The [`Tag`] parsed from the ilst atom, not guaranteed
	pub(crate) ilst: Option<Tag>,
	/// The file's audio properties
	pub(crate) properties: FileProperties,
}

impl AudioFile for Mp4File {
	fn read_from<R>(reader: &mut R) -> Result<Self>
	where
		R: Read + Seek,
	{
		self::read::read_from(reader)
	}

	fn properties(&self) -> &FileProperties {
		&self.properties
	}

	fn contains_tag(&self) -> bool {
		self.ilst.is_some()
	}

	fn contains_tag_type(&self, tag_type: &TagType) -> bool {
		match tag_type {
			TagType::Mp4Atom => self.ilst.is_some(),
			_ => false,
		}
	}
}

impl Mp4File {
	/// Returns a reference to the "ilst" tag if it exists
	pub fn ilst(&self) -> Option<&Tag> {
		self.ilst.as_ref()
	}
}
