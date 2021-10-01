mod constants;
pub(crate) mod header;
pub(crate) mod read;
pub(in crate::logic) mod write;

use crate::types::file::{AudioFile, FileType, TaggedFile};
use crate::{FileProperties, Result, Tag, TagType};
use header::{ChannelMode, Layer, MpegVersion};

use std::io::{Read, Seek};
use std::time::Duration;

/// An MP3 file's audio properties
pub struct Mp3Properties {
	version: MpegVersion,
	layer: Layer,
	channel_mode: ChannelMode,
	duration: Duration,
	bitrate: u32,
	sample_rate: u32,
	channels: u8,
}

impl From<Mp3Properties> for FileProperties {
	fn from(input: Mp3Properties) -> Self {
		Self {
			duration: input.duration,
			bitrate: Some(input.bitrate),
			sample_rate: Some(input.sample_rate),
			channels: Some(input.channels),
		}
	}
}

impl Mp3Properties {
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

	/// MPEG version
	pub fn version(&self) -> &MpegVersion {
		&self.version
	}

	/// MPEG layer
	pub fn layer(&self) -> &Layer {
		&self.layer
	}

	/// MPEG channel mode
	pub fn channel_mode(&self) -> &ChannelMode {
		&self.channel_mode
	}
}

/// An MP3 file
pub struct Mp3File {
	#[cfg(feature = "id3v2")]
	/// An ID3v2 tag
	pub(crate) id3v2: Option<Tag>,
	#[cfg(feature = "id3v1")]
	/// An ID3v1 tag
	pub(crate) id3v1: Option<Tag>,
	#[cfg(feature = "ape")]
	/// An APEv1/v2 tag
	pub(crate) ape: Option<Tag>,
	/// The file's audio properties
	pub(crate) properties: Mp3Properties,
}

impl From<Mp3File> for TaggedFile {
	fn from(input: Mp3File) -> Self {
		Self {
			ty: FileType::MP3,
			properties: FileProperties::from(input.properties),
			tags: vec![input.id3v1, input.id3v2, input.ape]
				.into_iter()
				.flatten()
				.collect(),
		}
	}
}

impl AudioFile for Mp3File {
	type Properties = Mp3Properties;

	fn read_from<R>(reader: &mut R) -> Result<Self>
	where
		R: Read + Seek,
	{
		self::read::read_from(reader)
	}

	fn properties(&self) -> &Self::Properties {
		&self.properties
	}

	fn contains_tag(&self) -> bool {
		self.id3v2.is_some() || self.id3v1.is_some() || self.ape.is_some()
	}

	fn contains_tag_type(&self, tag_type: &TagType) -> bool {
		match tag_type {
			TagType::Ape => self.ape.is_some(),
			TagType::Id3v2 => self.id3v2.is_some(),
			TagType::Id3v1 => self.id3v1.is_some(),
			_ => false,
		}
	}
}

impl Mp3File {
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
