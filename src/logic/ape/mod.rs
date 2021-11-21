mod constants;
mod properties;
pub(crate) mod read;
#[cfg(feature = "ape")]
pub(crate) mod tag;
pub(crate) mod write;

#[cfg(feature = "id3v1")]
use crate::logic::id3::v1::tag::Id3v1Tag;
use crate::logic::id3::v2::tag::Id3v2Tag;
use crate::logic::tag_methods;
use crate::types::file::{AudioFile, FileType, TaggedFile};
use crate::{FileProperties, Result, TagType};

use tag::ApeTag;

use std::io::{Read, Seek};
use std::time::Duration;

/// An APE file's audio properties
pub struct ApeProperties {
	version: u16,
	duration: Duration,
	bitrate: u32,
	sample_rate: u32,
	channels: u8,
}

impl From<ApeProperties> for FileProperties {
	fn from(input: ApeProperties) -> Self {
		Self {
			duration: input.duration,
			bitrate: Some(input.bitrate),
			sample_rate: Some(input.sample_rate),
			channels: Some(input.channels),
		}
	}
}

impl ApeProperties {
	/// Duration
	pub fn duration(&self) -> Duration {
		self.duration
	}

	/// Bitrate (kbps)
	pub fn bitrate(&self) -> u32 {
		self.bitrate
	}

	/// Sample rate (Hz)
	pub fn sample_rate(&self) -> u32 {
		self.sample_rate
	}

	/// Channel count
	pub fn channels(&self) -> u8 {
		self.channels
	}

	/// APE version
	pub fn version(&self) -> u16 {
		self.version
	}
}

/// An APE file
pub struct ApeFile {
	#[cfg(feature = "id3v1")]
	/// An ID3v1 tag
	pub(crate) id3v1_tag: Option<Id3v1Tag>,
	#[cfg(feature = "id3v2")]
	/// An ID3v2 tag (Not officially supported)
	pub(crate) id3v2_tag: Option<Id3v2Tag>,
	#[cfg(feature = "ape")]
	/// An APEv1/v2 tag
	pub(crate) ape_tag: Option<ApeTag>,
	/// The file's audio properties
	pub(crate) properties: ApeProperties,
}

impl From<ApeFile> for TaggedFile {
	fn from(input: ApeFile) -> Self {
		Self {
			ty: FileType::APE,
			properties: FileProperties::from(input.properties),
			tags: vec![
				input.ape_tag.map(|at| at.into()),
				input.id3v1_tag.map(|id3v1| id3v1.into()),
				input.id3v2_tag.map(|id3v2| id3v2.into()),
			]
			.into_iter()
			.flatten()
			.collect(),
		}
	}
}

impl AudioFile for ApeFile {
	type Properties = ApeProperties;

	fn read_from<R>(reader: &mut R) -> Result<Self>
	where
		R: Read + Seek,
		Self: Sized,
	{
		self::read::read_from(reader)
	}

	fn properties(&self) -> &Self::Properties {
		&self.properties
	}

	#[allow(clippy::match_same_arms)]
	fn contains_tag(&self) -> bool {
		match self {
			#[cfg(feature = "ape")]
			ApeFile {
				ape_tag: Some(_), ..
			} => true,
			#[cfg(feature = "id3v1")]
			ApeFile {
				id3v1_tag: Some(_), ..
			} => true,
			#[cfg(feature = "id3v2")]
			ApeFile {
				id3v2_tag: Some(_), ..
			} => true,
			_ => false,
		}
	}

	fn contains_tag_type(&self, tag_type: &TagType) -> bool {
		match tag_type {
			#[cfg(feature = "ape")]
			TagType::Ape => self.ape_tag.is_some(),
			#[cfg(feature = "id3v1")]
			TagType::Id3v1 => self.id3v1_tag.is_some(),
			#[cfg(feature = "id3v2")]
			TagType::Id3v2 => self.id3v2_tag.is_some(),
			_ => false,
		}
	}
}

tag_methods! {
	ApeFile => ID3v2, id3v2_tag, Id3v2Tag; ID3v1, id3v1_tag, Id3v1Tag; APE, ape_tag, ApeTag
}
