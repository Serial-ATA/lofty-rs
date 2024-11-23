use super::header::{ChannelMode, Emphasis, Header, Layer, MpegVersion, VbrHeader, VbrHeaderType};
use crate::error::Result;
use crate::mpeg::header::rev_search_for_frame_header;
use crate::properties::{ChannelMask, FileProperties};
use crate::util::math::RoundedDivision;

use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

/// An MPEG file's audio properties
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct MpegProperties {
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
	pub(crate) emphasis: Option<Emphasis>,
}

impl From<MpegProperties> for FileProperties {
	fn from(input: MpegProperties) -> Self {
		let MpegProperties {
			duration,
			overall_bitrate,
			audio_bitrate,
			sample_rate,
			channels,
			channel_mode,
			version: _,
			layer: _,
			copyright: _,
			emphasis: _,
			mode_extension: _,
			original: _,
		} = input;
		let channel_mask = match channel_mode {
			ChannelMode::SingleChannel => Some(ChannelMask::mono()),
			ChannelMode::Stereo | ChannelMode::JointStereo => Some(ChannelMask::stereo()),
			ChannelMode::DualChannel => None, // Cannot be represented by ChannelMask
		};
		Self {
			duration,
			overall_bitrate: Some(overall_bitrate),
			audio_bitrate: Some(audio_bitrate),
			sample_rate: Some(sample_rate),
			bit_depth: None,
			channels: Some(channels),
			channel_mask,
		}
	}
}

impl MpegProperties {
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
	pub fn emphasis(&self) -> Option<Emphasis> {
		self.emphasis
	}
}

pub(super) fn read_properties<R>(
	properties: &mut MpegProperties,
	reader: &mut R,
	first_frame: (Header, u64),
	mut last_frame_offset: u64,
	vbr_header: Option<VbrHeader>,
	file_length: u64,
) -> Result<()>
where
	R: Read + Seek,
{
	let first_frame_header = first_frame.0;
	let first_frame_offset = first_frame.1;

	properties.version = first_frame_header.version;
	properties.layer = first_frame_header.layer;
	properties.channel_mode = first_frame_header.channel_mode;
	properties.mode_extension = first_frame_header.mode_extension;
	properties.copyright = first_frame_header.copyright;
	properties.original = first_frame_header.original;
	properties.emphasis = first_frame_header.emphasis;
	properties.sample_rate = first_frame_header.sample_rate;
	properties.channels = if first_frame_header.channel_mode == ChannelMode::SingleChannel {
		1
	} else {
		2
	};

	if let Some(vbr_header) = vbr_header {
		if first_frame_header.sample_rate > 0 && vbr_header.is_valid() {
			log::debug!("MPEG: Valid VBR header; using it to calculate duration");

			let sample_rate = u64::from(first_frame_header.sample_rate);
			let samples_per_frame = u64::from(first_frame_header.samples);

			let total_frames = u64::from(vbr_header.frames);

			let length = (samples_per_frame * 1000 * total_frames).div_round(sample_rate);

			properties.duration = Duration::from_millis(length);
			properties.overall_bitrate = ((file_length * 8) / length) as u32;
			properties.audio_bitrate = ((u64::from(vbr_header.size) * 8) / length) as u32;

			return Ok(());
		}
	}

	// Nothing more we can do
	if first_frame_header.bitrate == 0 {
		return Ok(());
	}

	log::warn!("MPEG: Using bitrate to estimate duration");

	// http://gabriel.mp3-tech.org/mp3infotag.html:
	//
	// "In the Info Tag, the "Xing" identification string (mostly at 0x24) of the header is replaced by "Info" in case of a CBR file."
	let is_cbr = matches!(vbr_header.map(|h| h.ty), Some(VbrHeaderType::Info));
	if is_cbr {
		log::debug!("MPEG: CBR detected");
		properties.audio_bitrate = first_frame_header.bitrate;
	}

	// Search for the last frame, starting at the end of the frames
	reader.seek(SeekFrom::Start(last_frame_offset))?;

	let mut last_frame = None;
	let mut pos = last_frame_offset;
	while pos > 0 {
		match rev_search_for_frame_header(reader, &mut pos) {
			// Found a frame header
			Ok(Some(header)) => {
				// Move `last_frame_offset` back to the actual position
				last_frame_offset = pos;

				if header.cmp(&first_frame_header) {
					last_frame = Some(header);
					break;
				}
			},
			// Encountered some IO error, just break
			Err(_) => break,
			// No frame sync found, continue further back in the file
			_ => {},
		}
	}

	let Some(last_frame_header) = last_frame else {
		log::warn!("MPEG: Could not find last frame, properties will be incomplete");
		return Ok(());
	};

	let stream_end = last_frame_offset + u64::from(last_frame_header.len);
	if stream_end < first_frame_offset {
		// Something is incredibly wrong with this file, just give up
		return Ok(());
	}

	let stream_len = stream_end - first_frame_offset;
	if !is_cbr {
		log::debug!("MPEG: VBR detected");

		// TODO: Actually handle VBR streams, this still assumes CBR
		properties.audio_bitrate = first_frame_header.bitrate;
	}

	let length = (stream_len * 8).div_round(u64::from(properties.audio_bitrate));
	if length > 0 {
		properties.overall_bitrate = ((file_length * 8) / length) as u32;
		properties.duration = Duration::from_millis(length);
	}

	Ok(())
}
