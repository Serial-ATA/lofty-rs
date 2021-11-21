mod properties;
mod read;
#[cfg(feature = "aiff_text_chunks")]
pub(crate) mod tag;
pub(in crate::logic) mod write;

use crate::error::Result;
use crate::logic::id3::v2::tag::Id3v2Tag;
use crate::logic::tag_methods;
use crate::types::file::{AudioFile, FileType, TaggedFile};
use crate::types::properties::FileProperties;
use crate::types::tag::TagType;
use tag::AiffTextChunks;

use std::io::{Read, Seek};

/// An AIFF file
pub struct AiffFile {
	#[cfg(feature = "aiff_text_chunks")]
	/// Any text chunks included in the file
	pub(crate) text_chunks: Option<AiffTextChunks>,
	#[cfg(feature = "id3v2")]
	/// An ID3v2 tag
	pub(crate) id3v2_tag: Option<Id3v2Tag>,
	/// The file's audio properties
	pub(crate) properties: FileProperties,
}

impl From<AiffFile> for TaggedFile {
	fn from(input: AiffFile) -> Self {
		Self {
			ty: FileType::AIFF,
			properties: input.properties,
			tags: vec![
				input.text_chunks.map(|tc| tc.into()),
				input.id3v2_tag.map(|id3| id3.into()),
			]
			.into_iter()
			.flatten()
			.collect(),
		}
	}
}

impl AudioFile for AiffFile {
	type Properties = FileProperties;

	fn read_from<R>(reader: &mut R) -> Result<Self>
	where
		R: Read + Seek,
		Self: Sized,
	{
		read::read_from(reader)
	}

	fn properties(&self) -> &Self::Properties {
		&self.properties
	}

	fn contains_tag(&self) -> bool {
		self.id3v2_tag.is_some() || self.text_chunks.is_some()
	}

	fn contains_tag_type(&self, tag_type: &TagType) -> bool {
		match tag_type {
			TagType::Id3v2 => self.id3v2_tag.is_some(),
			TagType::AiffText => self.text_chunks.is_some(),
			_ => false,
		}
	}
}

tag_methods! {
	AiffFile => ID3v2, id3v2_tag, Id3v2Tag; Text_Chunks, text_chunks, AiffTextChunks
}
