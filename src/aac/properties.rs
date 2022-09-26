use crate::mpeg::header::MpegVersion;
use crate::properties::FileProperties;

use std::time::Duration;

#[derive(Default, Debug)]
#[allow(dead_code)] // TODO
pub struct AACProperties {
	pub(crate) version: MpegVersion,
	pub(crate) duration: Duration,
	pub(crate) overall_bitrate: u32,
	pub(crate) audio_bitrate: u32,
	pub(crate) sample_rate: u32,
	pub(crate) channels: u8,
	pub(crate) copyright: bool,
	pub(crate) original: bool,
}

impl From<AACProperties> for FileProperties {
	fn from(input: AACProperties) -> Self {
		FileProperties {
			duration: input.duration,
			overall_bitrate: Some(input.overall_bitrate),
			audio_bitrate: Some(input.audio_bitrate),
			sample_rate: Some(input.sample_rate),
			bit_depth: None,
			channels: Some(input.channels),
		}
	}
}
