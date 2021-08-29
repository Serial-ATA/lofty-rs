mod constants;
pub(crate) mod header;
pub(crate) mod read;

use crate::types::file::AudioFile;
use crate::{FileProperties, Result, Tag, TagType};

use std::io::{Read, Seek};

#[allow(dead_code)]
/// An MPEG file
pub struct MpegFile {
	/// An ID3v2 tag
	pub(crate) id3v2: Option<Tag>,
	/// An ID3v1 tag
	pub(crate) id3v1: Option<Tag>,
	/// An APEv1/v2 tag
	pub(crate) ape: Option<Tag>,
	/// The file's audio properties
	pub(crate) properties: FileProperties,
}

impl AudioFile for MpegFile {
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
		self.id3v2.is_some() || self.id3v1.is_some() || self.ape.is_some()
	}

	fn contains_tag_type(&self, tag_type: &TagType) -> bool {
		match tag_type {
			TagType::Ape => self.ape.is_some(),
			TagType::Id3v2(_) => self.id3v2.is_some(),
			TagType::Id3v1 => self.id3v1.is_some(),
			_ => false,
		}
	}
}

impl MpegFile {
	/// Returns a reference to the ID3v2 tag if it exists
	pub fn id3v2_tag(&self) -> Option<&Tag> {
		self.id3v2.as_ref()
	}

	/// Returns a reference to the ID3v1 tag if it exists
	pub fn id3v1_tag(&self) -> Option<&Tag> {
		self.id3v1.as_ref()
	}

	/// Returns a reference to the APEv1/2 tag if it exists
	pub fn ape_tag(&self) -> Option<&Tag> {
		self.ape.as_ref()
	}
}
