use crate::properties::{ChannelMask, FileProperties};
use crate::util::math::RoundedDivision;
use std::time::Duration;

/// DSF-specific audio properties
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DsfProperties {
	/// Sample rate (2822400 for DSD64, etc.)
	pub(crate) sample_rate: u32,
	/// Number of channels
	pub(crate) channels: u8,
	/// Bits per sample (1 or 8)
	pub(crate) bits_per_sample: u8,
	/// Total samples per channel
	pub(crate) sample_count: u64,
	/// Channel mask
	pub(crate) channel_mask: Option<ChannelMask>,
}

impl DsfProperties {
	/// Sample rate (Hz)
	pub fn sample_rate(&self) -> u32 {
		self.sample_rate
	}

	/// Number of channels
	pub fn channels(&self) -> u8 {
		self.channels
	}

	/// Bits per sample
	pub fn bits_per_sample(&self) -> u8 {
		self.bits_per_sample
	}

	/// Total samples per channel
	pub fn sample_count(&self) -> u64 {
		self.sample_count
	}

	/// Duration
	pub fn duration(&self) -> Duration {
		let duration_secs = self.sample_count as f64 / f64::from(self.sample_rate);
		Duration::from_secs_f64(duration_secs)
	}

	/// Audio bitrate (kbps)
	pub fn bitrate(&self) -> u32 {
		(u64::from(self.sample_rate) * u64::from(self.channels)).div_round(1000) as u32
	}
}

impl From<DsfProperties> for FileProperties {
	fn from(input: DsfProperties) -> Self {
		Self {
			duration: input.duration(),
			overall_bitrate: Some(input.bitrate()),
			audio_bitrate: Some(input.bitrate()),
			sample_rate: Some(input.sample_rate),
			bit_depth: Some(input.bits_per_sample),
			channels: Some(input.channels),
			channel_mask: input.channel_mask,
		}
	}
}
