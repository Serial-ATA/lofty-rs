mod constants;
pub(crate) mod header;
pub(crate) mod read;
pub(in crate::logic) mod write;

use crate::error::Result;
#[cfg(feature = "ape")]
use crate::logic::ape::tag::ape_tag::ApeTag;
#[cfg(feature = "id3v1")]
use crate::logic::id3::v1::tag::Id3v1Tag;
#[cfg(feature = "id3v2")]
use crate::logic::id3::v2::tag::Id3v2Tag;
use crate::logic::tag_methods;
use crate::types::file::{AudioFile, FileType, TaggedFile};
use crate::types::properties::FileProperties;
use crate::types::tag::{Tag, TagType};
use header::{ChannelMode, Layer, MpegVersion};

use std::io::{Read, Seek};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq)]
/// An MP3 file's audio properties
pub struct Mp3Properties {
	version: MpegVersion,
	layer: Layer,
	channel_mode: ChannelMode,
	duration: Duration,
	overall_bitrate: u32,
	audio_bitrate: u32,
	sample_rate: u32,
	channels: u8,
}

impl From<Mp3Properties> for FileProperties {
	fn from(input: Mp3Properties) -> Self {
		Self {
			duration: input.duration,
			overall_bitrate: Some(input.overall_bitrate),
			audio_bitrate: Some(input.audio_bitrate),
			sample_rate: Some(input.sample_rate),
			channels: Some(input.channels),
		}
	}
}

impl Mp3Properties {
	/// Creates a new [`Mp3Properties`]
	pub const fn new(
		version: MpegVersion,
		layer: Layer,
		channel_mode: ChannelMode,
		duration: Duration,
		overall_bitrate: u32,
		audio_bitrate: u32,
		sample_rate: u32,
		channels: u8,
	) -> Self {
		Self {
			version,
			layer,
			channel_mode,
			duration,
			overall_bitrate,
			audio_bitrate,
			sample_rate,
			channels,
		}
	}

	/// Duration
	pub fn duration(&self) -> Duration {
		self.duration
	}

	/// Overall bitrate (kbps)
	pub fn overall_bitrate(&self) -> u32 {
		self.overall_bitrate
	}

	/// Audio bitrate (kbps)
	pub fn audio_bitrate(&self) -> u32 {
		self.audio_bitrate
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
	pub(crate) id3v2_tag: Option<Id3v2Tag>,
	#[cfg(feature = "id3v1")]
	/// An ID3v1 tag
	pub(crate) id3v1_tag: Option<Id3v1Tag>,
	#[cfg(feature = "ape")]
	/// An APEv1/v2 tag
	pub(crate) ape_tag: Option<ApeTag>,
	/// The file's audio properties
	pub(crate) properties: Mp3Properties,
}

impl From<Mp3File> for TaggedFile {
	#[allow(clippy::vec_init_then_push)]
	fn from(input: Mp3File) -> Self {
		let mut tags = Vec::<Option<Tag>>::with_capacity(3);

		#[cfg(feature = "id3v2")]
		tags.push(input.id3v2_tag.map(Into::into));
		#[cfg(feature = "id3v1")]
		tags.push(input.id3v1_tag.map(Into::into));
		#[cfg(feature = "ape")]
		tags.push(input.ape_tag.map(Into::into));

		Self {
			ty: FileType::MP3,
			properties: FileProperties::from(input.properties),
			tags: tags.into_iter().flatten().collect(),
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

	#[allow(unreachable_code)]
	fn contains_tag(&self) -> bool {
		#[cfg(feature = "id3v2")]
		return self.id3v2_tag.is_some();
		#[cfg(feature = "id3v1")]
		return self.id3v1_tag.is_some();
		#[cfg(feature = "ape")]
		return self.ape_tag.is_some();

		false
	}

	fn contains_tag_type(&self, tag_type: &TagType) -> bool {
		match tag_type {
			#[cfg(feature = "ape")]
			TagType::Ape => self.ape_tag.is_some(),
			#[cfg(feature = "id3v2")]
			TagType::Id3v2 => self.id3v2_tag.is_some(),
			#[cfg(feature = "id3v1")]
			TagType::Id3v1 => self.id3v1_tag.is_some(),
			_ => false,
		}
	}
}

impl Mp3File {
	tag_methods! {
		#[cfg(feature = "id3v2")];
		ID3v2, id3v2_tag, Id3v2Tag;
		#[cfg(feature = "id3v1")];
		ID3v1, id3v1_tag, Id3v1Tag;
		#[cfg(feature = "ape")];
		APE, ape_tag, ApeTag
	}
}
