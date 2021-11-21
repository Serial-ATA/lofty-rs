use crate::error::{LoftyError, Result};
use crate::types::properties::FileProperties;

use std::time::Duration;

use byteorder::{LittleEndian, ReadBytesExt};

const PCM: u16 = 0x0001;
const IEEE_FLOAT: u16 = 0x0003;
const EXTENSIBLE: u16 = 0xfffe;

#[allow(missing_docs, non_camel_case_types)]
/// A WAV file's format
pub enum WavFormat {
	PCM,
	IEEE_FLOAT,
	Other(u16),
}

/// A WAV file's audio properties
pub struct WavProperties {
	format: WavFormat,
	duration: Duration,
	bitrate: u32,
	sample_rate: u32,
	channels: u8,
}

impl From<WavProperties> for FileProperties {
	fn from(input: WavProperties) -> Self {
		Self {
			duration: input.duration,
			bitrate: Some(input.bitrate),
			sample_rate: Some(input.sample_rate),
			channels: Some(input.channels),
		}
	}
}

impl WavProperties {
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

	/// WAV format
	pub fn format(&self) -> &WavFormat {
		&self.format
	}
}

pub(super) fn read_properties(
	fmt: &mut &[u8],
	total_samples: u32,
	stream_len: u32,
) -> Result<WavProperties> {
	let mut format_tag = fmt.read_u16::<LittleEndian>()?;
	let channels = fmt.read_u16::<LittleEndian>()? as u8;

	if channels == 0 {
		return Err(LoftyError::Wav("File contains 0 channels"));
	}

	let sample_rate = fmt.read_u32::<LittleEndian>()?;
	let bytes_per_second = fmt.read_u32::<LittleEndian>()?;

	// Skip 2 bytes
	// Block align (2)
	let _ = fmt.read_u16::<LittleEndian>()?;

	let bits_per_sample = fmt.read_u16::<LittleEndian>()?;

	if format_tag == EXTENSIBLE {
		if fmt.len() < 40 {
			return Err(LoftyError::Wav(
				"Extensible format identified, invalid \"fmt \" chunk size found (< 40)",
			));
		}

		// Skip 8 bytes
		// cbSize (Size of extra format information) (2)
		// Valid bits per sample (2)
		// Channel mask (4)
		let _ = fmt.read_u64::<LittleEndian>()?;

		format_tag = fmt.read_u16::<LittleEndian>()?;
	}

	let non_pcm = format_tag != PCM && format_tag != IEEE_FLOAT;

	if non_pcm && total_samples == 0 {
		return Err(LoftyError::Wav(
			"Non-PCM format identified, no \"fact\" chunk found",
		));
	}

	let sample_frames = if non_pcm {
		total_samples
	} else if bits_per_sample > 0 {
		stream_len / u32::from(u16::from(channels) * ((bits_per_sample + 7) / 8))
	} else {
		0
	};

	let (duration, bitrate) = if sample_rate > 0 && sample_frames > 0 {
		let length = (u64::from(sample_frames) * 1000) / u64::from(sample_rate);

		(
			Duration::from_millis(length),
			(u64::from(stream_len * 8) / length) as u32,
		)
	} else if bytes_per_second > 0 {
		let length = (u64::from(stream_len) * 1000) / u64::from(bytes_per_second);

		(Duration::from_millis(length), (bytes_per_second * 8) / 1000)
	} else {
		(Duration::ZERO, 0)
	};

	Ok(WavProperties {
		format: match format_tag {
			PCM => WavFormat::PCM,
			IEEE_FLOAT => WavFormat::IEEE_FLOAT,
			other => WavFormat::Other(other),
		},
		duration,
		bitrate,
		sample_rate,
		channels,
	})
}
