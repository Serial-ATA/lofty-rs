mod constants;
mod properties;
pub(crate) mod read;
pub(crate) mod tag;
pub(crate) mod write;

use crate::types::file::AudioFile;
use crate::{FileProperties, Result, Tag, TagType};

use std::io::{Read, Seek};

/// An APE file
pub struct ApeFile {
	#[cfg(feature = "id3v1")]
	/// An ID3v1 tag
	pub(crate) id3v1: Option<Tag>,
	#[cfg(feature = "id3v2")]
	/// An ID3v2 tag (Not officially supported)
	pub(crate) id3v2: Option<Tag>,
	#[cfg(feature = "ape")]
	/// An APEv1/v2 tag
	pub(crate) ape: Option<Tag>,
	/// The file's audio properties
	pub(crate) properties: FileProperties,
}

impl AudioFile for ApeFile {
	fn read_from<R>(reader: &mut R) -> Result<Self>
	where
		R: Read + Seek,
		Self: Sized,
	{
		self::read::read_from(reader)
	}

	fn properties(&self) -> &FileProperties {
		&self.properties
	}

	fn contains_tag(&self) -> bool {
		self.ape.is_some() || self.id3v1.is_some() || self.id3v2.is_some()
	}

	fn contains_tag_type(&self, tag_type: &TagType) -> bool {
		match tag_type {
			TagType::Ape => self.ape.is_some(),
			TagType::Id3v1 => self.id3v1.is_some(),
			TagType::Id3v2(_) => self.id3v2.is_some(),
			_ => false,
		}
	}
}

impl ApeFile {
	#[cfg(feature = "id3v2")]
	/// Returns a reference to the ID3v2 tag if it exists
	pub fn id3v2_tag(&self) -> Option<&Tag> {
		self.id3v2.as_ref()
	}

	#[cfg(feature = "id3v2")]
	/// Returns a mutable reference to the ID3v2 tag if it exists
	pub fn id3v2_tag_mut(&mut self) -> Option<&mut Tag> {
		self.id3v2.as_mut()
	}

	#[cfg(feature = "id3v1")]
	/// Returns a reference to the ID3v1 tag if it exists
	pub fn id3v1_tag(&self) -> Option<&Tag> {
		self.id3v1.as_ref()
	}

	#[cfg(feature = "id3v1")]
	/// Returns a mutable reference to the ID3v1 tag if it exists
	pub fn id3v1_tag_mut(&mut self) -> Option<&mut Tag> {
		self.id3v1.as_mut()
	}

	#[cfg(feature = "ape")]
	/// Returns a reference to the APEv1/2 tag if it exists
	pub fn ape_tag(&self) -> Option<&Tag> {
		self.ape.as_ref()
	}

	#[cfg(feature = "ape")]
	/// Returns a mutable reference to the APEv1/2 tag if it exists
	pub fn ape_tag_mut(&mut self) -> Option<&mut Tag> {
		self.ape.as_mut()
	}
}
