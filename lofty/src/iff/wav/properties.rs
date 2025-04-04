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

#[derive(Copy, Clone, Debug)]
struct ExtensibleFmtChunk {
	valid_bits_per_sample: u16,
	channel_mask: ChannelMask,
}

#[derive(Copy, Clone, Debug)]
struct FmtChunk {
	format_tag: u16,
	channels: u8,
	sample_rate: u32,
	bytes_per_second: u32,
	block_align: u16,
	bits_per_sample: u16,
	extensible_info: Option<ExtensibleFmtChunk>,
}

fn read_fmt_chunk<R>(reader: &mut R, len: usize) -> Result<FmtChunk>
where
	R: ReadBytesExt,
{
	let format_tag = reader.read_u16::<LittleEndian>()?;
	let channels = reader.read_u16::<LittleEndian>()?;
	let sample_rate = reader.read_u32::<LittleEndian>()?;
	let bytes_per_second = reader.read_u32::<LittleEndian>()?;
	let block_align = reader.read_u16::<LittleEndian>()?;
	let bits_per_sample = reader.read_u16::<LittleEndian>()?;

	let mut fmt_chunk = FmtChunk {
		format_tag,
		channels: channels as u8,
		sample_rate,
		bytes_per_second,
		block_align,
		bits_per_sample,
		extensible_info: None,
	};

	if format_tag == EXTENSIBLE {
		if len < 40 {
			decode_err!(@BAIL Wav, "Extensible format identified, invalid \"fmt \" chunk size found (< 40)");
		}

		// cbSize (Size of extra format information) (2)
		let _cb_size = reader.read_u16::<LittleEndian>()?;

		// Valid bits per sample (2)
		let valid_bits_per_sample = reader.read_u16::<LittleEndian>()?;

		// Channel mask (4)
		let channel_mask = ChannelMask(reader.read_u32::<LittleEndian>()?);

		fmt_chunk.format_tag = reader.read_u16::<LittleEndian>()?;
		fmt_chunk.extensible_info = Some(ExtensibleFmtChunk {
			valid_bits_per_sample,
			channel_mask,
		});
	}

	Ok(fmt_chunk)
}

pub(super) fn read_properties(
	fmt: &mut &[u8],
	mut total_samples: u32,
	stream_len: u32,
	file_length: u64,
) -> Result<WavProperties> {
	if fmt.len() < 16 {
		decode_err!(@BAIL Wav, "File does not contain a valid \"fmt \" chunk");
	}

	if stream_len == 0 {
		decode_err!(@BAIL Wav, "File does not contain a \"data\" chunk");
	}

	let FmtChunk {
		format_tag,
		channels,
		sample_rate,
		bytes_per_second,
		block_align,
		bits_per_sample,
		extensible_info,
	} = read_fmt_chunk(fmt, fmt.len())?;

	if channels == 0 {
		decode_err!(@BAIL Wav, "File contains 0 channels");
	}

	if bits_per_sample % 8 != 0 {
		decode_err!(@BAIL Wav, "Bits per sample is not a multiple of 8");
	}

	let bytes_per_sample = block_align / u16::from(channels);

	let bit_depth;
	match extensible_info {
		Some(ExtensibleFmtChunk {
			valid_bits_per_sample,
			..
		}) if valid_bits_per_sample > 0 => bit_depth = valid_bits_per_sample as u8,
		_ if bits_per_sample > 0 => bit_depth = bits_per_sample as u8,
		_ => bit_depth = bytes_per_sample.saturating_mul(8) as u8,
	}

	let channel_mask = extensible_info.map(|info| info.channel_mask);

	let pcm = format_tag == PCM || format_tag == IEEE_FLOAT;
	if !pcm && total_samples == 0 {
		decode_err!(@BAIL Wav, "Non-PCM format identified, no \"fact\" chunk found");
	}

	if bits_per_sample > 0 && (total_samples == 0 || pcm) {
		total_samples = stream_len / (u32::from(channels) * u32::from(bits_per_sample / 8));
	}

	let mut duration = Duration::ZERO;
	let mut overall_bitrate = 0;
	let mut audio_bitrate = 0;
	if bytes_per_second > 0 {
		audio_bitrate = (u64::from(bytes_per_second) * 8).div_round(1000) as u32;
	}

	if sample_rate > 0 && total_samples > 0 {
		log::debug!("Calculating duration and bitrate from total samples");

		let length = (u64::from(total_samples) * 1000).div_round(u64::from(sample_rate));
		duration = Duration::from_millis(length);
		if length > 0 {
			overall_bitrate = (file_length * 8).div_round(length) as u32;
			if audio_bitrate == 0 {
				log::warn!("Estimating audio bitrate from stream length");
				audio_bitrate = (u64::from(stream_len) * 8).div_round(length) as u32;
			}
		}
	} else if stream_len > 0 && bytes_per_second > 0 {
		log::debug!("Calculating duration and bitrate from stream length/byte rate");

		let length = (u64::from(stream_len) * 1000).div_round(u64::from(bytes_per_second));
		duration = Duration::from_millis(length);
		if length > 0 {
			overall_bitrate = (file_length * 8).div_round(length) as u32;
		}
	} else {
		log::warn!("Unable to calculate duration and bitrate");
	}

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
