use crate::properties::FileProperties;

use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[non_exhaustive]
/// A WavPack file's audio properties
pub struct WavPackProperties {
	pub(crate) version: u16,
	pub(crate) duration: Duration,
	pub(crate) overall_bitrate: u32,
	pub(crate) audio_bitrate: u32,
	pub(crate) sample_rate: u32,
	pub(crate) channels: u8,
	pub(crate) bit_depth: u8,
	pub(crate) lossless: bool,
}

impl From<WavPackProperties> for FileProperties {
	fn from(input: WavPackProperties) -> Self {
		Self {
			duration: input.duration,
			overall_bitrate: Some(input.overall_bitrate),
			audio_bitrate: Some(input.audio_bitrate),
			sample_rate: Some(input.sample_rate),
			bit_depth: Some(input.bit_depth),
			channels: Some(input.channels),
		}
	}
}

impl WavPackProperties {
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

	/// WavPack version
	pub fn version(&self) -> u16 {
		self.version
	}

	/// Whether the audio is lossless
	pub fn is_lossless(&self) -> bool {
		self.lossless
	}
}
