mod properties;
mod read;
pub(crate) mod write;

use crate::error::Result;
use crate::file::{AudioFile, FileType, TaggedFile};
#[cfg(feature = "id3v2")]
use crate::id3::v2::tag::ID3v2Tag;
use crate::properties::FileProperties;
use crate::tag::{Tag, TagType};

use std::io::{Read, Seek};

cfg_if::cfg_if! {
	if #[cfg(feature = "aiff_text_chunks")] {
		pub(crate) mod tag;
		use tag::AIFFTextChunks;
	}
}

/// An AIFF file
pub struct AiffFile {
	#[cfg(feature = "aiff_text_chunks")]
	/// Any text chunks included in the file
	pub(crate) text_chunks: Option<AIFFTextChunks>,
	#[cfg(feature = "id3v2")]
	/// An ID3v2 tag
	pub(crate) id3v2_tag: Option<ID3v2Tag>,
	/// The file's audio properties
	pub(crate) properties: FileProperties,
}

impl From<AiffFile> for TaggedFile {
	#[allow(unused_mut)]
	fn from(input: AiffFile) -> Self {
		let mut tags = Vec::<Option<Tag>>::with_capacity(3);

		#[cfg(feature = "aiff_text_chunks")]
		tags.push(input.text_chunks.map(Into::into));
		#[cfg(feature = "id3v2")]
		tags.push(input.id3v2_tag.map(Into::into));

		Self {
			ty: FileType::AIFF,
			properties: input.properties,
			tags: tags.into_iter().flatten().collect(),
		}
	}
}

impl AudioFile for AiffFile {
	type Properties = FileProperties;

	fn read_from<R>(reader: &mut R, read_properties: bool) -> Result<Self>
	where
		R: Read + Seek,
		Self: Sized,
	{
		read::read_from(reader, read_properties)
	}

	fn properties(&self) -> &Self::Properties {
		&self.properties
	}

	#[allow(unreachable_code)]
	fn contains_tag(&self) -> bool {
		#[cfg(feature = "id3v2")]
		return self.id3v2_tag.is_some();
		#[cfg(feature = "aiff_text_chunks")]
		return self.text_chunks.is_some();

		false
	}

	fn contains_tag_type(&self, tag_type: TagType) -> bool {
		match tag_type {
			#[cfg(feature = "id3v2")]
			TagType::ID3v2 => self.id3v2_tag.is_some(),
			#[cfg(feature = "aiff_text_chunks")]
			TagType::AIFFText => self.text_chunks.is_some(),
			_ => false,
		}
	}
}

impl AiffFile {
	crate::macros::tag_methods! {
		#[cfg(feature = "id3v2")]
		id3v2_tag, ID3v2Tag;

		#[cfg(feature = "aiff_text_chunks")]
		text_chunks, AIFFTextChunks
	}
}
