use crate::properties::{ChannelMask, FileProperties};

use std::time::Duration;

/// DSF audio properties
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct DsfProperties {
	pub(crate) duration: Duration,
	pub(crate) overall_bitrate: u32,
	pub(crate) audio_bitrate: u32,
	pub(crate) sample_rate: u32,
	pub(crate) bits_per_sample: u8,
	pub(crate) channels: u8,
	pub(crate) channel_mask: Option<ChannelMask>,
}

impl From<DsfProperties> for FileProperties {
	fn from(input: DsfProperties) -> Self {
		Self {
			duration: input.duration,
			overall_bitrate: Some(input.overall_bitrate),
			audio_bitrate: Some(input.audio_bitrate),
			sample_rate: Some(input.sample_rate),
			bit_depth: Some(input.bits_per_sample),
			channels: Some(input.channels),
			channel_mask: input.channel_mask,
		}
	}
}

impl DsfProperties {
	/// Duration of the audio
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
	///
	/// Common DSD sample rates:
	/// - DSD64: 2,822,400 Hz
	/// - DSD128: 5,644,800 Hz
	/// - DSD256: 11,289,600 Hz
	/// - DSD512: 22,579,200 Hz
	pub fn sample_rate(&self) -> u32 {
		self.sample_rate
	}

	/// Bits per sample (1 for DSD, or 8 when stored as packed bytes)
	pub fn bits_per_sample(&self) -> u8 {
		self.bits_per_sample
	}

	/// Number of channels
	pub fn channels(&self) -> u8 {
		self.channels
	}

	/// Channel mask, if available
	pub fn channel_mask(&self) -> Option<&ChannelMask> {
		self.channel_mask.as_ref()
	}
}
