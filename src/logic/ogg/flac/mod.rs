mod block;
mod read;
pub(crate) mod write;

use crate::error::Result;
use crate::types::file::{AudioFile, FileType, TaggedFile};
use crate::types::item::{ItemKey, ItemValue, TagItem};
use crate::types::properties::FileProperties;
use crate::types::tag::{Tag, TagType};

use std::io::{Read, Seek};

/// A FLAC file
pub struct FlacFile {
	/// The file's audio properties
	pub(crate) properties: FileProperties,
	#[cfg(feature = "vorbis_comments")]
	/// The file vendor's name found in the vorbis comments (if it exists)
	pub(crate) vendor: Option<String>,
	#[cfg(feature = "vorbis_comments")]
	/// The vorbis comments contained in the file
	///
	/// NOTE: This field being `Some` does not mean the file has vorbis comments, as Picture blocks exist.
	pub(crate) vorbis_comments: Option<Tag>,
}

impl From<FlacFile> for TaggedFile {
	fn from(input: FlacFile) -> Self {
		// Preserve vendor string
		let tags = {
			if let Some(mut tag) = input.vorbis_comments {
				if let Some(vendor) = input.vendor {
					tag.insert_item_unchecked(TagItem::new(
						ItemKey::EncoderSoftware,
						ItemValue::Text(vendor),
					))
				}

				vec![tag]
			} else {
				Vec::new()
			}
		};

		Self {
			ty: FileType::FLAC,
			properties: input.properties,
			tags,
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
		if tag_type != &TagType::VorbisComments {
			return false;
		}

		self.vorbis_comments.is_some()
	}
}

impl FlacFile {
	#[cfg(feature = "vorbis_comments")]
	/// Returns a reference to the Vorbis comments tag if it exists
	pub fn vorbis_comments(&self) -> Option<&Tag> {
		self.vorbis_comments.as_ref()
	}

	#[cfg(feature = "vorbis_comments")]
	/// Returns a mutable reference to the Vorbis comments tag if it exists
	pub fn vorbis_comments_mut(&mut self) -> Option<&mut Tag> {
		self.vorbis_comments.as_mut()
	}
}
