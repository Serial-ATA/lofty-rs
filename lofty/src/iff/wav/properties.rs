use crate::error::Result;
use crate::macros::decode_err;
use crate::properties::{ChannelMask, FileProperties};
use crate::util::math::RoundedDivision;

use std::time::Duration;

use byteorder::{LittleEndian, ReadBytesExt};

const PCM: u16 = 0x0001;
const IEEE_FLOAT: u16 = 0x0003;
const EXTENSIBLE: u16 = 0xFFFE;

/// A WAV file's format
#[allow(missing_docs, non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WavFormat {
	PCM,
	IEEE_FLOAT,
	Other(u16),
}

impl Default for WavFormat {
	fn default() -> Self {
		Self::Other(0)
	}
}

/// A WAV file's audio properties
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct WavProperties {
	pub(crate) format: WavFormat,
	pub(crate) duration: Duration,
	pub(crate) overall_bitrate: u32,
	pub(crate) audio_bitrate: u32,
	pub(crate) sample_rate: u32,
	pub(crate) bit_depth: u8,
	pub(crate) channels: u8,
	pub(crate) channel_mask: Option<ChannelMask>,
}

impl From<WavProperties> for FileProperties {
	fn from(input: WavProperties) -> Self {
		let WavProperties {
			duration,
			overall_bitrate,
			audio_bitrate,
			sample_rate,
			bit_depth,
			channels,
			channel_mask,
			format: _,
		} = input;
		Self {
			duration,
			overall_bitrate: Some(overall_bitrate),
			audio_bitrate: Some(audio_bitrate),
			sample_rate: Some(sample_rate),
			bit_depth: Some(bit_depth),
			channels: Some(channels),
			channel_mask,
		}
	}
}

impl WavProperties {
	/// Duration of the audio
	pub fn duration(&self) -> Duration {
		self.duration
	}

	/// Overall bitrate (kbps)
	pub fn overall_bitrate(&self) -> u32 {
		self.overall_bitrate
	}

	/// Audio bitrate (kbps)
	pub fn bitrate(&self) -> u32 {
		self.audio_bitrate
	}

	/// Sample rate (Hz)
	pub fn sample_rate(&self) -> u32 {
		self.sample_rate
	}

	/// Bits per sample
	pub fn bit_depth(&self) -> u8 {
		self.bit_depth
	}

	/// Channel count
	pub fn channels(&self) -> u8 {
		self.channels
	}

	/// Channel mask
	pub fn channel_mask(&self) -> Option<ChannelMask> {
		self.channel_mask
	}

	/// WAV format
	pub fn format(&self) -> &WavFormat {
		&self.format
	}
}

pub(super) fn read_properties(
	fmt: &mut &[u8],
	mut total_samples: u32,
	stream_len: u32,
	file_length: u64,
) -> Result<WavProperties> {
	let mut format_tag = fmt.read_u16::<LittleEndian>()?;
	let channels = fmt.read_u16::<LittleEndian>()? as u8;

	if channels == 0 {
		decode_err!(@BAIL Wav, "File contains 0 channels");
	}

	let sample_rate = fmt.read_u32::<LittleEndian>()?;
	let bytes_per_second = fmt.read_u32::<LittleEndian>()?;

	let block_align = fmt.read_u16::<LittleEndian>()?;

	let bits_per_sample = fmt.read_u16::<LittleEndian>()?;
	let bytes_per_sample = block_align / u16::from(channels);

	let mut bit_depth = if bits_per_sample > 0 {
		bits_per_sample as u8
	} else {
		(bytes_per_sample * 8) as u8
	};

	let channel_mask;
	if format_tag == EXTENSIBLE {
		if fmt.len() + 16 < 40 {
			decode_err!(@BAIL Wav, "Extensible format identified, invalid \"fmt \" chunk size found (< 40)");
		}

		// cbSize (Size of extra format information) (2)
		let _cb_size = fmt.read_u16::<LittleEndian>()?;
		// Valid bits per sample (2)
		let valid_bits_per_sample = fmt.read_u16::<LittleEndian>()?;
		// Channel mask (4)
		channel_mask = Some(ChannelMask(fmt.read_u32::<LittleEndian>()?));

		if valid_bits_per_sample > 0 {
			bit_depth = valid_bits_per_sample as u8;
		}
		format_tag = fmt.read_u16::<LittleEndian>()?;
	} else {
		channel_mask = None;
	}

	let non_pcm = format_tag != PCM && format_tag != IEEE_FLOAT;

	if non_pcm && total_samples == 0 {
		decode_err!(@BAIL Wav, "Non-PCM format identified, no \"fact\" chunk found");
	}

	if bits_per_sample > 0 {
		total_samples = stream_len / u32::from(u16::from(channels) * ((bits_per_sample + 7) / 8))
	} else if !non_pcm {
		total_samples = 0
	}

	let duration;
	let overall_bitrate;
	let audio_bitrate;
	if sample_rate > 0 && total_samples > 0 {
		let length = (u64::from(total_samples) * 1000).div_round(u64::from(sample_rate));
		if length == 0 {
			duration = Duration::ZERO;
			overall_bitrate = 0;
			audio_bitrate = 0;
		} else {
			duration = Duration::from_millis(length);
			overall_bitrate = (file_length * 8).div_round(length) as u32;
			audio_bitrate = (u64::from(stream_len) * 8).div_round(length) as u32;
		}
	} else if stream_len > 0 && bytes_per_second > 0 {
		let length = (u64::from(stream_len) * 1000).div_round(u64::from(bytes_per_second));
		if length == 0 {
			duration = Duration::ZERO;
			overall_bitrate = 0;
			audio_bitrate = 0;
		} else {
			duration = Duration::from_millis(length);
			overall_bitrate = (file_length * 8).div_round(length) as u32;
			audio_bitrate = (bytes_per_second * 8).div_round(1000);
		}
	} else {
		duration = Duration::ZERO;
		overall_bitrate = 0;
		audio_bitrate = 0;
	};

	Ok(WavProperties {
		format: match format_tag {
			PCM => WavFormat::PCM,
			IEEE_FLOAT => WavFormat::IEEE_FLOAT,
			other => WavFormat::Other(other),
		},
		duration,
		overall_bitrate,
		audio_bitrate,
		sample_rate,
		bit_depth,
		channels,
		channel_mask,
	})
}
