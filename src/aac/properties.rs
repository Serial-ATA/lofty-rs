use crate::aac::header::ADTSHeader;
use crate::mp4::AudioObjectType;
use crate::mpeg::header::MpegVersion;
use crate::properties::FileProperties;

use std::time::Duration;

/// An AAC file's audio properties
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AACProperties {
	pub(crate) version: MpegVersion,
	pub(crate) audio_object_type: AudioObjectType,
	pub(crate) duration: Duration,
	pub(crate) overall_bitrate: u32,
	pub(crate) audio_bitrate: u32,
	pub(crate) sample_rate: u32,
	pub(crate) channels: u8,
	pub(crate) copyright: bool,
	pub(crate) original: bool,
}

impl AACProperties {
	/// MPEG version
	///
	/// The only possible variants are:
	///
	/// * [MpegVersion::V2]
	/// * [MpegVersion::V4]
	#[must_use]
	pub fn version(&self) -> MpegVersion {
		self.version
	}

	/// Audio object type
	///
	/// The only possible variants are:
	///
	/// * [AudioObjectType::AacMain]
	/// * [AudioObjectType::AacLowComplexity]
	/// * [AudioObjectType::AacScalableSampleRate]
	/// * [AudioObjectType::AacLongTermPrediction]
	#[must_use]
	pub fn audio_object_type(&self) -> AudioObjectType {
		self.audio_object_type
	}

	/// Duration of the audio
	#[must_use]
	pub fn duration(&self) -> Duration {
		self.duration
	}

	/// Overall bitrate (kbps)
	#[must_use]
	pub fn overall_bitrate(&self) -> u32 {
		self.overall_bitrate
	}

	/// Audio bitrate (kbps)
	#[must_use]
	pub fn audio_bitrate(&self) -> u32 {
		self.audio_bitrate
	}

	/// Sample rate (Hz)
	#[must_use]
	pub fn sample_rate(&self) -> u32 {
		self.sample_rate
	}

	/// Channel count
	#[must_use]
	pub fn channels(&self) -> u8 {
		self.channels
	}

	/// Whether the audio is copyrighted
	#[must_use]
	pub fn copyright(&self) -> bool {
		self.copyright
	}

	/// Whether the media is original or a copy
	#[must_use]
	pub fn original(&self) -> bool {
		self.original
	}
}

impl From<AACProperties> for FileProperties {
	fn from(input: AACProperties) -> Self {
		FileProperties {
			duration: input.duration,
			overall_bitrate: Some(input.overall_bitrate),
			audio_bitrate: Some(input.audio_bitrate),
			sample_rate: Some(input.sample_rate),
			bit_depth: None,
			channels: Some(input.channels),
			channel_mask: None,
		}
	}
}

pub(super) fn read_properties(
	properties: &mut AACProperties,
	first_frame: ADTSHeader,
	stream_len: u64,
) {
	properties.version = first_frame.version;
	properties.audio_object_type = first_frame.audio_object_ty;
	properties.sample_rate = first_frame.sample_rate;
	properties.channels = first_frame.channels;
	properties.copyright = first_frame.copyright;
	properties.original = first_frame.original;

	let bitrate = first_frame.bitrate;

	if bitrate > 0 {
		properties.audio_bitrate = bitrate;
		properties.overall_bitrate = bitrate;
		properties.duration = Duration::from_millis((stream_len * 8) / u64::from(bitrate));
	}
}
