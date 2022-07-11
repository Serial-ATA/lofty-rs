//! WavPack specific items
mod properties;
mod read;
pub(crate) mod write;

#[cfg(feature = "ape")]
use crate::ape::tag::ApeTag;
use crate::error::Result;
use crate::file::{AudioFile, FileType, TaggedFile};
#[cfg(feature = "id3v1")]
use crate::id3::v1::tag::ID3v1Tag;
use crate::properties::FileProperties;
use crate::tag::{Tag, TagType};

use std::io::{Read, Seek};

// Exports
pub use properties::WavPackProperties;

/// A WavPack file
#[derive(Default)]
pub struct WavPackFile {
	#[cfg(feature = "id3v1")]
	/// An ID3v1 tag
	pub(crate) id3v1_tag: Option<ID3v1Tag>,
	#[cfg(feature = "ape")]
	/// An APEv1/v2 tag
	pub(crate) ape_tag: Option<ApeTag>,
	/// The file's audio properties
	pub(crate) properties: WavPackProperties,
}

impl From<WavPackFile> for TaggedFile {
	#[allow(clippy::vec_init_then_push, unused_mut)]
	fn from(input: WavPackFile) -> Self {
		let mut tags = Vec::<Option<Tag>>::with_capacity(2);

		#[cfg(feature = "id3v1")]
		tags.push(input.id3v1_tag.map(Into::into));
		#[cfg(feature = "ape")]
		tags.push(input.ape_tag.map(Into::into));

		Self {
			ty: FileType::WavPack,
			properties: FileProperties::from(input.properties),
			tags: tags.into_iter().flatten().collect(),
		}
	}
}

impl AudioFile for WavPackFile {
	type Properties = WavPackProperties;

	fn read_from<R>(reader: &mut R, read_properties: bool) -> Result<Self>
	where
		R: Read + Seek,
	{
		read::read_from(reader, read_properties)
	}

	fn properties(&self) -> &Self::Properties {
		&self.properties
	}

	#[allow(unreachable_code)]
	fn contains_tag(&self) -> bool {
		#[cfg(feature = "id3v1")]
		return self.id3v1_tag.is_some();
		#[cfg(feature = "ape")]
		return self.ape_tag.is_some();

		false
	}

	fn contains_tag_type(&self, tag_type: TagType) -> bool {
		match tag_type {
			#[cfg(feature = "ape")]
			TagType::APE => self.ape_tag.is_some(),
			#[cfg(feature = "id3v1")]
			TagType::ID3v1 => self.id3v1_tag.is_some(),
			_ => false,
		}
	}
}

impl WavPackFile {
	crate::macros::tag_methods! {
		#[cfg(feature = "id3v1")]
		id3v1_tag, ID3v1Tag;

		#[cfg(feature = "ape")]
		ape_tag, ApeTag
	}
}
