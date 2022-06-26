//! Items for FLAC
//!
//! ## File notes
//!
//! * See [`FlacFile`]

mod block;
mod properties;
mod read;
#[cfg(feature = "vorbis_comments")]
pub(crate) mod write;

use crate::error::Result;
use crate::file::{AudioFile, FileType, TaggedFile};
#[cfg(feature = "id3v2")]
use crate::id3::v2::tag::ID3v2Tag;
#[cfg(feature = "vorbis_comments")]
use crate::ogg::VorbisComments;
use crate::properties::FileProperties;
use crate::tag::{Tag, TagType};

use std::io::{Read, Seek};

/// A FLAC file
///
/// ## Notes
///
/// * The ID3v2 tag is **read only**, and it's use is discouraged by spec
/// * Picture blocks will be stored in the `VorbisComments` tag, meaning a file could have no vorbis
///   comments block, but `FlacFile::vorbis_comments` will exist.
///   * When writing, the pictures will be stored in their own picture blocks
///   * This behavior will likely change in the future
pub struct FlacFile {
	#[cfg(feature = "id3v2")]
	/// An ID3v2 tag
	pub(crate) id3v2_tag: Option<ID3v2Tag>,
	#[cfg(feature = "vorbis_comments")]
	/// The vorbis comments contained in the file
	///
	/// NOTE: This field being `Some` does not mean the file has vorbis comments, as Picture blocks exist.
	pub(crate) vorbis_comments: Option<VorbisComments>,
	/// The file's audio properties
	pub(crate) properties: FileProperties,
}

impl From<FlacFile> for TaggedFile {
	#[allow(clippy::vec_init_then_push)]
	fn from(input: FlacFile) -> Self {
		let mut tags = Vec::<Option<Tag>>::with_capacity(2);

		#[cfg(feature = "vorbis_comments")]
		tags.push(input.vorbis_comments.map(Into::into));
		#[cfg(feature = "id3v2")]
		tags.push(input.id3v2_tag.map(Into::into));

		Self {
			ty: FileType::FLAC,
			properties: input.properties,
			#[cfg(any(feature = "vorbis_comments", feature = "id3v2"))]
			tags: tags.into_iter().flatten().collect(),
			#[cfg(not(any(feature = "vorbis_comments", feature = "id3v2")))]
			tags: Vec::new(),
		}
	}
}

impl AudioFile for FlacFile {
	type Properties = FileProperties;

	fn read_from<R>(reader: &mut R, read_properties: bool) -> Result<Self>
	where
		R: Read + Seek,
	{
		read::read_from(reader, read_properties)
	}

	fn properties(&self) -> &Self::Properties {
		&self.properties
	}

	fn contains_tag(&self) -> bool {
		#[cfg(feature = "vorbis_comments")]
		return self.vorbis_comments.is_some();

		#[cfg(not(feature = "vorbis_comments"))]
		return false;
	}

	#[allow(unused_variables)]
	fn contains_tag_type(&self, tag_type: TagType) -> bool {
		#[cfg(feature = "vorbis_comments")]
		return tag_type == TagType::VorbisComments && self.vorbis_comments.is_some();

		#[cfg(not(feature = "vorbis_comments"))]
		return false;
	}
}

impl FlacFile {
	crate::macros::tag_methods! {
		#[cfg(feature = "vorbis_comments")]
		vorbis_comments, VorbisComments;

		#[cfg(feature = "id3v2")]
		id3v2_tag, ID3v2Tag
	}
}
