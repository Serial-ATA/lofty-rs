use std::time::Duration;

#[derive(Debug, PartialEq, Clone)]
/// Various *immutable* audio properties
pub struct FileProperties {
	pub(crate) duration: Duration,
	pub(crate) overall_bitrate: Option<u32>,
	pub(crate) audio_bitrate: Option<u32>,
	pub(crate) sample_rate: Option<u32>,
	pub(crate) bit_depth: Option<u8>,
	pub(crate) channels: Option<u8>,
}

impl Default for FileProperties {
	fn default() -> Self {
		Self {
			duration: Duration::ZERO,
			overall_bitrate: None,
			audio_bitrate: None,
			sample_rate: None,
			bit_depth: None,
			channels: None,
		}
	}
}

impl FileProperties {
	/// Creates a new [`FileProperties`]
	pub const fn new(
		duration: Duration,
		overall_bitrate: Option<u32>,
		audio_bitrate: Option<u32>,
		sample_rate: Option<u32>,
		bit_depth: Option<u8>,
		channels: Option<u8>,
	) -> Self {
		Self {
			duration,
			overall_bitrate,
			audio_bitrate,
			sample_rate,
			bit_depth,
			channels,
		}
	}

	/// Duration
	pub fn duration(&self) -> Duration {
		self.duration
	}

	/// Overall bitrate (kbps)
	pub fn overall_bitrate(&self) -> Option<u32> {
		self.overall_bitrate
	}

	/// Audio bitrate (kbps)
	pub fn audio_bitrate(&self) -> Option<u32> {
		self.audio_bitrate
	}

	/// Sample rate (Hz)
	pub fn sample_rate(&self) -> Option<u32> {
		self.sample_rate
	}

	/// Bits per sample (usually 16 or 24 bit)
	pub fn bit_depth(&self) -> Option<u8> {
		self.bit_depth
	}

	/// Channel count
	pub fn channels(&self) -> Option<u8> {
		self.channels
	}
}
