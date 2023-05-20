use crate::error::Result;
use crate::probe::ParsingMode;
use crate::properties::FileProperties;

use std::io::Read;
use std::time::Duration;

/// MPC stream versions 4-6 audio properties
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MpcSv4to6Properties {
	pub(crate) duration: Duration,
	pub(crate) overall_bitrate: u32,
	pub(crate) audio_bitrate: u32,
	pub(crate) channels: u8,     // NOTE: always 2
	pub(crate) sample_rate: u32, // NOTE: always 44100
	frame_count: u32,
}

impl From<MpcSv4to6Properties> for FileProperties {
	fn from(input: MpcSv4to6Properties) -> Self {
		Self {
			duration: input.duration,
			overall_bitrate: Some(input.overall_bitrate),
			audio_bitrate: Some(input.audio_bitrate),
			sample_rate: Some(input.sample_rate),
			bit_depth: None,
			channels: Some(input.channels),
			channel_mask: None,
		}
	}
}

impl MpcSv4to6Properties {
	pub(crate) fn read<R>(_reader: &mut R, _parse_mode: ParsingMode) -> Result<Self>
	where
		R: Read,
	{
		todo!()
	}
}
