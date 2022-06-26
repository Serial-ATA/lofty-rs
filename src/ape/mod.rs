//! APE specific items
//!
//! ## File notes
//!
//! It is possible for an `APE` file to contain an `ID3v2` tag. For the sake of data preservation,
//! this tag will be read, but **cannot** be written. The only tags allowed by spec are `APEv1/2` and
//! `ID3v1`.
pub(crate) mod constants;
pub(crate) mod header;
mod properties;
mod read;
pub(crate) mod write;

use crate::error::Result;
use crate::file::{AudioFile, FileType, TaggedFile};
#[cfg(feature = "id3v1")]
use crate::id3::v1::tag::Id3v1Tag;
#[cfg(feature = "id3v2")]
use crate::id3::v2::tag::ID3v2Tag;
use crate::properties::FileProperties;
use crate::tag::{Tag, TagType};

use std::io::{Read, Seek};

// Exports

cfg_if::cfg_if! {
	if #[cfg(feature = "ape")] {
		pub(crate) mod tag;
		pub use tag::ApeTag;
		pub use tag::item::ApeItem;

		pub use crate::picture::APE_PICTURE_TYPES;
	}
}

pub use properties::ApeProperties;

/// An APE file
pub struct ApeFile {
	#[cfg(feature = "id3v1")]
	/// An ID3v1 tag
	pub(crate) id3v1_tag: Option<Id3v1Tag>,
	#[cfg(feature = "id3v2")]
	/// An ID3v2 tag (Not officially supported)
	pub(crate) id3v2_tag: Option<ID3v2Tag>,
	#[cfg(feature = "ape")]
	/// An APEv1/v2 tag
	pub(crate) ape_tag: Option<ApeTag>,
	/// The file's audio properties
	pub(crate) properties: ApeProperties,
}

impl From<ApeFile> for TaggedFile {
	#[allow(clippy::vec_init_then_push, unused_mut)]
	fn from(input: ApeFile) -> Self {
		let mut tags = Vec::<Option<Tag>>::with_capacity(3);

		#[cfg(feature = "ape")]
		tags.push(input.ape_tag.map(Into::into));
		#[cfg(feature = "id3v1")]
		tags.push(input.id3v1_tag.map(Into::into));
		#[cfg(feature = "id3v2")]
		tags.push(input.id3v2_tag.map(Into::into));

		Self {
			ty: FileType::APE,
			properties: FileProperties::from(input.properties),
			tags: tags.into_iter().flatten().collect(),
		}
	}
}

impl AudioFile for ApeFile {
	type Properties = ApeProperties;

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
		#[cfg(feature = "ape")]
		return self.ape_tag.is_some();
		#[cfg(feature = "id3v1")]
		return self.id3v1_tag.is_some();
		#[cfg(feature = "id3v2")]
		return self.id3v2_tag.is_some();

		false
	}

	fn contains_tag_type(&self, tag_type: TagType) -> bool {
		match tag_type {
			#[cfg(feature = "ape")]
			TagType::APE => self.ape_tag.is_some(),
			#[cfg(feature = "id3v1")]
			TagType::ID3v1 => self.id3v1_tag.is_some(),
			#[cfg(feature = "id3v2")]
			TagType::ID3v2 => self.id3v2_tag.is_some(),
			_ => false,
		}
	}
}

impl ApeFile {
	crate::macros::tag_methods! {
		#[cfg(feature = "id3v2")]
		id3v2_tag, ID3v2Tag;

		#[cfg(feature = "id3v1")]
		id3v1_tag, Id3v1Tag;

		#[cfg(feature = "ape")]
		ape_tag, ApeTag
	}
}
