pub(in crate::logic::ogg) mod properties;
pub(in crate::logic::ogg) mod write;

use super::find_last_page;
use crate::error::Result;
use crate::logic::ogg::constants::{OPUSHEAD, OPUSTAGS};
use crate::types::file::AudioFile;
use crate::types::properties::FileProperties;
use crate::types::tag::{Tag, TagType};

use std::io::{Read, Seek};

/// An OGG Opus file
pub struct OpusFile {
	/// The file's audio properties
	pub(crate) properties: FileProperties,
	#[cfg(feature = "vorbis_comments")]
	/// The file vendor's name
	pub(crate) vendor: String,
	#[cfg(feature = "vorbis_comments")]
	/// The vorbis comments contained in the file
	///
	/// NOTE: While a metadata packet is required, it isn't required to actually have any data.
	pub(crate) vorbis_comments: Tag,
}

impl AudioFile for OpusFile {
	fn read_from<R>(reader: &mut R) -> Result<Self>
	where
		R: Read + Seek,
	{
		let file_information = super::read::read_from(reader, OPUSHEAD, OPUSTAGS)?;

		Ok(Self {
			properties: file_information.2,
			vendor: file_information.0,
			vorbis_comments: file_information.1,
		})
	}

	fn properties(&self) -> &FileProperties {
		&self.properties
	}

	fn contains_tag(&self) -> bool {
		true
	}

	fn contains_tag_type(&self, tag_type: &TagType) -> bool {
		if tag_type != &TagType::VorbisComments {
			return false;
		}

		true
	}
}

impl OpusFile {
	#[cfg(feature = "vorbis_comments")]
	/// Returns a reference to the Vorbis comments tag
	pub fn vorbis_comments(&self) -> &Tag {
		&self.vorbis_comments
	}
}
