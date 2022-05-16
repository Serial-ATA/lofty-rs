use super::header::{ChannelMode, Emphasis, Header, Layer, MpegVersion, XingHeader};
use crate::error::Result;
use crate::mp3::header::{cmp_header, rev_search_for_frame_sync, HeaderCmpResult};
use crate::properties::FileProperties;

use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

use byteorder::{BigEndian, ReadBytesExt};

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

pub(super) fn read_properties<R>(
	reader: &mut R,
	first_frame: (Header, u64),
	mut last_frame_offset: u64,
	xing_header: Option<XingHeader>,
	file_length: u64,
) -> Result<Mp3Properties>
where
	R: Read + Seek,
{
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
		channels: if first_frame_header.channel_mode == ChannelMode::SingleChannel {
			1
		} else {
			2
		},
		emphasis: first_frame_header.emphasis,
	};

	match xing_header {
		Some(xing_header) if first_frame_header.sample_rate > 0 && xing_header.is_valid() => {
			let frame_time =
				u32::from(first_frame_header.samples) * 1000 / first_frame_header.sample_rate;
			let length = u64::from(frame_time) * u64::from(xing_header.frames);

			properties.duration = Duration::from_millis(length);
			properties.overall_bitrate = ((file_length * 8) / length) as u32;
			properties.audio_bitrate = ((u64::from(xing_header.size) * 8) / length) as u32;
		},
		_ if first_frame_header.bitrate > 0 => {
			properties.audio_bitrate = first_frame_header.bitrate;

			// Search for the last frame, starting at the end of the frames
			reader.seek(SeekFrom::Start(last_frame_offset))?;

			let mut last_frame = None;
			let mut pos = reader.stream_position()?;
			while pos > 0 {
				match rev_search_for_frame_sync(reader, &mut pos) {
					// Found a frame sync, attempt to read a header
					Ok(Some(_)) => {
						// Move `last_frame_offset` back to the actual position
						last_frame_offset = reader.stream_position()?;
						let last_frame_data = reader.read_u32::<BigEndian>()?;

						if let Some(last_frame_header) = Header::read(last_frame_data) {
							match cmp_header(reader, last_frame_header.len, last_frame_data) {
								HeaderCmpResult::Equal | HeaderCmpResult::Undetermined => {
									last_frame = Some(last_frame_header);
									break;
								},
								HeaderCmpResult::NotEqual => {},
							}
						}
					},
					// Encountered some IO error, just break
					Err(_) => break,
					// No frame sync found, continue further back in the file
					_ => {},
				}
			}

			if let Some(last_frame_header) = last_frame {
				let stream_len =
					last_frame_offset - first_frame_offset + u64::from(last_frame_header.len);
				let length = (stream_len * 8) / u64::from(properties.audio_bitrate);

				if length > 0 {
					properties.overall_bitrate = ((file_length * 8) / length) as u32;
					properties.duration = Duration::from_millis(length);
				}
			}
		},
		_ => {},
	}

	Ok(properties)
}
