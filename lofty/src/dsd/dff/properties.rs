use crate::properties::FileProperties;
use crate::util::math::RoundedDivision;

use std::time::Duration;

/// Loudspeaker configuration for DFF files
///
/// As defined in the DSDIFF 1.5 specification § 3.2.5
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoudspeakerConfig {
	/// 2-channel stereo setup
	Stereo,
	/// 5-channel setup according to ITU-R BS.775-1
	FiveChannel,
	/// 6-channel setup (5.1 configuration)
	///
	/// 5-channel setup according to ITU-R BS.775-1, plus Low Frequency Enhancement (LFE)
	FivePointOne,
	/// Undefined channel setup
	Undefined,
	/// Reserved value for future use
	Reserved(u16),
}

impl LoudspeakerConfig {
	/// Create a loudspeaker config from the raw value
	pub(crate) fn from_u16(value: u16) -> Self {
		match value {
			0 => Self::Stereo,
			3 => Self::FiveChannel,
			4 => Self::FivePointOne,
			65535 => Self::Undefined,
			_ => Self::Reserved(value),
		}
	}

	/// Convert to raw u16 value
	pub fn to_u16(self) -> u16 {
		match self {
			Self::Stereo => 0,
			Self::FiveChannel => 3,
			Self::FivePointOne => 4,
			Self::Undefined => 65535,
			Self::Reserved(v) => v,
		}
	}
}

/// DFF (DSDIFF) audio properties
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DffProperties {
	sample_rate: u32,
	channels: u8,
	/// Duration in seconds
	duration: Duration,
	/// Number of DSD samples per channel
	sample_count: u64,
	/// Compression type (usually "DSD " for uncompressed)
	compression: Option<String>,
	/// Loudspeaker configuration
	loudspeaker_config: Option<LoudspeakerConfig>,
}

impl DffProperties {
	/// Create new DFF properties
	pub(crate) fn new(
		sample_rate: u32,
		channels: u8,
		sample_count: u64,
		compression: Option<String>,
		loudspeaker_config: Option<LoudspeakerConfig>,
	) -> Self {
		let duration = if sample_rate > 0 {
			let seconds = sample_count as f64 / f64::from(sample_rate);
			Duration::from_secs_f64(seconds)
		} else {
			Duration::ZERO
		};

		Self {
			sample_rate,
			channels,
			duration,
			sample_count,
			compression,
			loudspeaker_config,
		}
	}

	/// Sample rate in Hz
	pub fn sample_rate(&self) -> u32 {
		self.sample_rate
	}

	/// Number of channels
	pub fn channels(&self) -> u8 {
		self.channels
	}

	/// Duration of the audio
	pub fn duration(&self) -> Duration {
		self.duration
	}

	/// Total number of DSD samples per channel
	pub fn sample_count(&self) -> u64 {
		self.sample_count
	}

	/// Compression type (usually "DSD " for uncompressed DSD)
	pub fn compression(&self) -> Option<&str> {
		self.compression.as_deref()
	}

	/// Loudspeaker configuration
	pub fn loudspeaker_config(&self) -> Option<LoudspeakerConfig> {
		self.loudspeaker_config
	}
}

impl From<DffProperties> for FileProperties {
	fn from(props: DffProperties) -> Self {
		// Calculate bitrate: sample_rate * channels (1 bit per sample)
		let bitrate = if props.sample_rate > 0 && props.channels > 0 {
			Some((u64::from(props.sample_rate) * u64::from(props.channels)).div_round(1000) as u32)
		} else {
			None
		};

		Self::new(
			props.duration,
			bitrate, // overall_bitrate
			bitrate, // audio_bitrate (same as overall for DSD)
			Some(props.sample_rate),
			Some(1), // bit_depth - DSD is 1-bit
			Some(props.channels),
			None, // channel_mask
		)
	}
}
