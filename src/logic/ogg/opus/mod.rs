pub(in crate::logic::ogg) mod properties;
pub(in crate::logic::ogg) mod write;

use super::find_last_page;
use crate::error::Result;
use crate::logic::ogg::constants::{OPUSHEAD, OPUSTAGS};
use crate::types::file::{AudioFile, FileType, TaggedFile};
use crate::types::item::{ItemKey, ItemValue, TagItem};
use crate::types::properties::FileProperties;
use crate::types::tag::{Tag, TagType};

use std::io::{Read, Seek};
use std::time::Duration;

/// An Opus file's audio properties
pub struct OpusProperties {
	duration: Duration,
	bitrate: u32,
	channels: u8,
	version: u8,
	input_sample_rate: u32,
}

impl From<OpusProperties> for FileProperties {
	fn from(input: OpusProperties) -> Self {
		Self {
			duration: input.duration,
			bitrate: Some(input.bitrate),
			sample_rate: Some(input.input_sample_rate),
			channels: Some(input.channels),
		}
	}
}

impl OpusProperties {
	/// Duration
	pub fn duration(&self) -> Duration {
		self.duration
	}

	/// Bitrate (kbps)
	pub fn bitrate(&self) -> u32 {
		self.bitrate
	}

	/// Channel count
	pub fn channels(&self) -> u8 {
		self.channels
	}

	/// Opus version
	pub fn version(&self) -> u8 {
		self.version
	}

	/// Input sample rate
	pub fn input_sample_rate(&self) -> u32 {
		self.input_sample_rate
	}
}

/// An OGG Opus file
pub struct OpusFile {
	#[cfg(feature = "vorbis_comments")]
	/// The file vendor's name
	pub(crate) vendor: String,
	#[cfg(feature = "vorbis_comments")]
	/// The vorbis comments contained in the file
	///
	/// NOTE: While a metadata packet is required, it isn't required to actually have any data.
	pub(crate) vorbis_comments: Tag,
	/// The file's audio properties
	pub(crate) properties: OpusProperties,
}

impl From<OpusFile> for TaggedFile {
	fn from(input: OpusFile) -> Self {
		// Preserve vendor string
		let mut tag = input.vorbis_comments;

		if !input.vendor.is_empty() {
			tag.insert_item_unchecked(TagItem::new(
				ItemKey::EncoderSoftware,
				ItemValue::Text(input.vendor),
			))
		}

		Self {
			ty: FileType::Opus,
			properties: FileProperties::from(input.properties),
			tags: vec![tag],
		}
	}
}

impl AudioFile for OpusFile {
	type Properties = OpusProperties;

	fn read_from<R>(reader: &mut R) -> Result<Self>
	where
		R: Read + Seek,
	{
		let file_information = super::read::read_from(reader, OPUSHEAD, OPUSTAGS)?;

		Ok(Self {
			properties: properties::read_properties(reader, &file_information.2)?,
			vendor: file_information.0,
			vorbis_comments: file_information.1,
		})
	}

	fn properties(&self) -> &Self::Properties {
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

	#[cfg(feature = "vorbis_comments")]
	/// Returns a mutable reference to the Vorbis comments tag
	pub fn vorbis_comments_mut(&mut self) -> &mut Tag {
		&mut self.vorbis_comments
	}
}
