mod block;
mod read;
pub(crate) mod write;

use crate::error::Result;
use crate::types::file::AudioFile;
use crate::types::properties::FileProperties;
use crate::types::tag::{Tag, TagType};

use std::io::{Read, Seek};

/// A FLAC file
pub struct FlacFile {
	/// The file's audio properties
	pub(crate) properties: FileProperties,
	/// The file vendor's name found in the vorbis comments (if it exists)
	pub(crate) vendor: Option<String>,
	/// The vorbis comments contained in the file
	///
	/// NOTE: This field being `Some` does not mean the file has vorbis comments, as Picture blocks exist.
	pub(crate) metadata: Option<Tag>,
}

impl AudioFile for FlacFile {
	fn read_from<R>(reader: &mut R) -> Result<Self>
	where
		R: Read + Seek,
	{
		read::read_from(reader)
	}

	fn properties(&self) -> &FileProperties {
		&self.properties
	}

	fn contains_tag(&self) -> bool {
		self.metadata.is_some()
	}

	fn contains_tag_type(&self, tag_type: &TagType) -> bool {
		if tag_type != &TagType::VorbisComments {
			return false;
		}

		self.metadata.is_some()
	}
}
