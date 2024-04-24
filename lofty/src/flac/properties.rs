use crate::error::Result;
use crate::properties::FileProperties;

use std::io::Read;
use std::time::Duration;

use byteorder::{BigEndian, ReadBytesExt};

/// A FLAC file's audio properties
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct FlacProperties {
	pub(crate) duration: Duration,
	pub(crate) overall_bitrate: u32,
	pub(crate) audio_bitrate: u32,
	pub(crate) sample_rate: u32,
	pub(crate) bit_depth: u8,
	pub(crate) channels: u8,
	pub(crate) signature: u128,
}

impl From<FlacProperties> for FileProperties {
	fn from(input: FlacProperties) -> Self {
		Self {
			duration: input.duration,
			overall_bitrate: Some(input.overall_bitrate),
			audio_bitrate: Some(input.audio_bitrate),
			sample_rate: Some(input.sample_rate),
			bit_depth: Some(input.bit_depth),
			channels: Some(input.channels),
			channel_mask: None,
		}
	}
}

impl FlacProperties {
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
	pub fn sample_rate(&self) -> u32 {
		self.sample_rate
	}

	/// Bits per sample (usually 16 or 24 bit)
	pub fn bit_depth(&self) -> u8 {
		self.bit_depth
	}

	/// Channel count
	pub fn channels(&self) -> u8 {
		self.channels
	}

	/// MD5 signature of the unencoded audio data
	pub fn signature(&self) -> u128 {
		self.signature
	}
}

pub(crate) fn read_properties<R>(
	stream_info: &mut R,
	stream_length: u64,
	file_length: u64,
) -> Result<FlacProperties>
where
	R: Read,
{
	// Skip 4 bytes
	// Minimum block size (2)
	// Maximum block size (2)
	stream_info.read_u32::<BigEndian>()?;

	// Skip 6 bytes
	// Minimum frame size (3)
	// Maximum frame size (3)
	stream_info.read_uint::<BigEndian>(6)?;

	// Read 4 bytes
	// Sample rate (20 bits)
	// Number of channels (3 bits)
	// Bits per sample (5 bits)
	// Total samples (first 4 bits)
	let info = stream_info.read_u32::<BigEndian>()?;

	let sample_rate = info >> 12;
	let bits_per_sample = ((info >> 4) & 0b11111) + 1;
	let channels = ((info >> 9) & 7) + 1;

	// Read the remaining 32 bits of the total samples
	let total_samples = stream_info.read_u32::<BigEndian>()? | (info << 28);

	let signature = stream_info.read_u128::<BigEndian>()?;

	let mut properties = FlacProperties {
		sample_rate,
		bit_depth: bits_per_sample as u8,
		channels: channels as u8,
		signature,
		..FlacProperties::default()
	};

	if sample_rate > 0 && total_samples > 0 {
		let length = (u64::from(total_samples) * 1000) / u64::from(sample_rate);
		properties.duration = Duration::from_millis(length);

		if length > 0 && file_length > 0 && stream_length > 0 {
			properties.overall_bitrate = ((file_length * 8) / length) as u32;
			properties.audio_bitrate = ((stream_length * 8) / length) as u32;
		}
	}

	Ok(properties)
}
