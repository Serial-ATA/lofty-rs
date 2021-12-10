mod block;
mod read;
pub(crate) mod write;

use super::tag::VorbisComments;
use crate::error::Result;
use crate::logic::tag_methods;
use crate::types::file::{AudioFile, FileType, TaggedFile};
use crate::types::properties::FileProperties;
use crate::types::tag::TagType;

use std::io::{Read, Seek};

/// A FLAC file
pub struct FlacFile {
	#[cfg(feature = "vorbis_comments")]
	/// The vorbis comments contained in the file
	///
	/// NOTE: This field being `Some` does not mean the file has vorbis comments, as Picture blocks exist.
	pub(crate) vorbis_comments: Option<VorbisComments>,
	/// The file's audio properties
	pub(crate) properties: FileProperties,
}

impl From<FlacFile> for TaggedFile {
	fn from(input: FlacFile) -> Self {
		Self {
			ty: FileType::FLAC,
			properties: input.properties,
			tags: input
				.vorbis_comments
				.map_or_else(Vec::new, |t| vec![t.into()]),
		}
	}
}

impl AudioFile for FlacFile {
	type Properties = FileProperties;

	fn read_from<R>(reader: &mut R) -> Result<Self>
	where
		R: Read + Seek,
	{
		read::read_from(reader)
	}

	fn properties(&self) -> &Self::Properties {
		&self.properties
	}

	fn contains_tag(&self) -> bool {
		self.vorbis_comments.is_some()
	}

	fn contains_tag_type(&self, tag_type: &TagType) -> bool {
		tag_type == &TagType::VorbisComments && self.vorbis_comments.is_some()
	}
}

tag_methods! {
	FlacFile => Vorbis_Comments, vorbis_comments, VorbisComments
}
