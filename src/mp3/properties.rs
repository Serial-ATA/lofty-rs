use super::header::{ChannelMode, Emphasis, Header, Layer, MpegVersion, XingHeader};
use crate::types::properties::FileProperties;

use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[non_exhaustive]
/// An MP3 file's audio properties
pub struct Mp3Properties {
	pub(crate) version: MpegVersion,
	pub(crate) layer: Layer,
	pub(crate) duration: Duration,
	pub(crate) overall_bitrate: u32,
	pub(crate) audio_bitrate: u32,
	pub(crate) sample_rate: u32,
	pub(crate) channels: u8,
	pub(crate) channel_mode: ChannelMode,
	pub(crate) mode_extension: Option<u8>,
	pub(crate) copyright: bool,
	pub(crate) original: bool,
	pub(crate) emphasis: Emphasis,
}

impl From<Mp3Properties> for FileProperties {
	fn from(input: Mp3Properties) -> Self {
		Self {
			duration: input.duration,
			overall_bitrate: Some(input.overall_bitrate),
			audio_bitrate: Some(input.audio_bitrate),
			sample_rate: Some(input.sample_rate),
			bit_depth: None,
			channels: Some(input.channels),
		}
	}
}

impl Mp3Properties {
	/// Duration
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

	/// Channel count
	pub fn channels(&self) -> u8 {
		self.channels
	}

	/// MPEG version
	pub fn version(&self) -> &MpegVersion {
		&self.version
	}

	/// MPEG layer
	pub fn layer(&self) -> &Layer {
		&self.layer
	}

	/// MPEG channel mode
	pub fn channel_mode(&self) -> &ChannelMode {
		&self.channel_mode
	}

	/// A channel mode extension specifically for [`ChannelMode::JointStereo`]
	pub fn mode_extension(&self) -> Option<u8> {
		self.mode_extension
	}

	/// Whether the audio is copyrighted
	pub fn is_copyright(&self) -> bool {
		self.copyright
	}

	/// Whether the media is original or a copy
	pub fn is_original(&self) -> bool {
		self.original
	}

	/// See [`Emphasis`]
	pub fn emphasis(&self) -> Emphasis {
		self.emphasis
	}
}

pub(super) fn read_properties(
	first_frame: (Header, u64),
	last_frame_offset: u64,
	xing_header: Option<XingHeader>,
	file_length: u64,
) -> Mp3Properties {
	let first_frame_header = first_frame.0;
	let first_frame_offset = first_frame.1;

	let mut properties = Mp3Properties {
		version: first_frame_header.version,
		layer: first_frame_header.layer,
		channel_mode: first_frame_header.channel_mode,
		mode_extension: first_frame_header.mode_extension,
		copyright: first_frame_header.copyright,
		original: first_frame_header.original,
		duration: Duration::ZERO,
		overall_bitrate: 0,
		audio_bitrate: 0,
		sample_rate: first_frame_header.sample_rate,
		channels: first_frame_header.channels as u8,
		emphasis: first_frame_header.emphasis,
	};

	match xing_header {
		Some(xing_header) if first_frame_header.sample_rate > 0 => {
			let frame_time =
				u32::from(first_frame_header.samples) * 1000 / first_frame_header.sample_rate;
			let length = u64::from(frame_time) * u64::from(xing_header.frames);

			properties.duration = Duration::from_millis(length);
			properties.overall_bitrate = ((file_length * 8) / length) as u32;
			properties.audio_bitrate = ((u64::from(xing_header.size) * 8) / length) as u32;
		},
		_ if first_frame_header.bitrate > 0 => {
			let audio_bitrate = first_frame_header.bitrate;

			let stream_length =
				last_frame_offset - first_frame_offset + u64::from(first_frame_header.len);
			let length = (stream_length * 8) / u64::from(audio_bitrate);

			properties.audio_bitrate = audio_bitrate;
			properties.overall_bitrate = ((file_length * 8) / length) as u32;
			properties.duration = Duration::from_millis(length);
		},
		_ => {},
	}

	properties
}
