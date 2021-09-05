mod read;
pub(in crate::logic) mod write;

use crate::error::Result;
use crate::types::file::AudioFile;
use crate::types::properties::FileProperties;
use crate::types::tag::{Tag, TagType};

use std::io::{Read, Seek};

/// A WAV file
pub struct WavFile {
	/// The file's audio properties
	pub(crate) properties: FileProperties,
	/// A RIFF INFO LIST
	pub(crate) riff_info: Option<Tag>,
	/// An ID3v2 tag
	pub(crate) id3v2: Option<Tag>,
}

impl AudioFile for WavFile {
	fn read_from<R>(reader: &mut R) -> Result<Self>
	where
		R: Read + Seek,
		Self: Sized,
	{
		read::read_from(reader)
	}

	fn properties(&self) -> &FileProperties {
		&self.properties
	}

	fn contains_tag(&self) -> bool {
		self.id3v2.is_some() || self.riff_info.is_some()
	}

	fn contains_tag_type(&self, tag_type: &TagType) -> bool {
		match tag_type {
			TagType::Id3v2(_) => self.id3v2.is_some(),
			TagType::RiffInfo => self.riff_info.is_some(),
			_ => false,
		}
	}
}

impl WavFile {
	/// Returns a reference to the ID3v2 tag if it exists
	pub fn id3v2_tag(&self) -> Option<&Tag> {
		self.id3v2.as_ref()
	}

	/// Returns a reference to the RIFF INFO tag if it exists
	pub fn riff_info(&self) -> Option<&Tag> {
		self.riff_info.as_ref()
	}
}
