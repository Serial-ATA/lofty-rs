use crate::config::ParsingMode;
use crate::error::Result;
use crate::macros::decode_err;
use crate::properties::FileProperties;

use std::io::Read;
use std::time::Duration;

use aud_io::math::RoundedDivision;
use aud_io::musepack::sv8::{EncoderInfo, ReplayGain, StreamHeader};

/// MPC stream version 8 audio properties
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MpcSv8Properties {
	pub(crate) duration: Duration,
	pub(crate) average_bitrate: u32,
	/// Mandatory Stream Header packet
	pub stream_header: StreamHeader,
	/// Mandatory ReplayGain packet
	pub replay_gain: ReplayGain,
	/// Optional encoder information
	pub encoder_info: Option<EncoderInfo>,
}

impl From<MpcSv8Properties> for FileProperties {
	fn from(input: MpcSv8Properties) -> Self {
		Self {
			duration: input.duration,
			overall_bitrate: Some(input.average_bitrate),
			audio_bitrate: Some(input.average_bitrate),
			sample_rate: Some(input.stream_header.sample_rate),
			bit_depth: None,
			channels: Some(input.stream_header.channels),
			channel_mask: None,
		}
	}
}

impl MpcSv8Properties {
	/// Duration of the audio
	pub fn duration(&self) -> Duration {
		self.duration
	}

	/// Average bitrate (kbps)
	pub fn average_bitrate(&self) -> u32 {
		self.average_bitrate
	}

	/// Sample rate (Hz)
	pub fn sample_rate(&self) -> u32 {
		self.stream_header.sample_rate
	}

	/// Channel count
	pub fn channels(&self) -> u8 {
		self.stream_header.channels
	}

	/// MusePack stream version
	pub fn version(&self) -> u8 {
		self.stream_header.stream_version
	}

	pub(crate) fn read<R: Read>(reader: &mut R, parse_mode: ParsingMode) -> Result<Self> {
		super::read::read_from(reader, parse_mode)
	}
}

pub(super) fn read(
	stream_length: u64,
	stream_header: StreamHeader,
	replay_gain: ReplayGain,
	encoder_info: Option<EncoderInfo>,
) -> Result<MpcSv8Properties> {
	let mut properties = MpcSv8Properties {
		duration: Duration::ZERO,
		average_bitrate: 0,
		stream_header,
		replay_gain,
		encoder_info,
	};

	let sample_count = stream_header.sample_count;
	let beginning_silence = stream_header.beginning_silence;
	let sample_rate = stream_header.sample_rate;

	if beginning_silence > sample_count {
		decode_err!(@BAIL Mpc, "Beginning silence is greater than the total sample count");
	}

	if sample_rate == 0 {
		log::warn!("Sample rate is 0, unable to calculate duration and bitrate");
		return Ok(properties);
	}

	if sample_count == 0 {
		log::warn!("Sample count is 0, unable to calculate duration and bitrate");
		return Ok(properties);
	}

	let total_samples = sample_count - beginning_silence;
	if total_samples == 0 {
		log::warn!(
			"Sample count (after removing beginning silence) is 0, unable to calculate duration \
			 and bitrate"
		);
		return Ok(properties);
	}

	let length = (total_samples * 1000).div_round(u64::from(sample_rate));

	properties.duration = Duration::from_millis(length);
	properties.average_bitrate =
		((stream_length * 8 * u64::from(sample_rate)) / (total_samples * 1000)) as u32;

	Ok(properties)
}
