use super::find_last_page;
use crate::error::Result;
use crate::macros::decode_err;
use crate::properties::{ChannelMask, FileProperties};
use crate::util::math::RoundedDivision;

use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

use byteorder::{LittleEndian, ReadBytesExt};
use ogg_pager::{Packets, PageHeader};

/// An Opus file's audio properties
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct OpusProperties {
	pub(crate) duration: Duration,
	pub(crate) overall_bitrate: u32,
	pub(crate) audio_bitrate: u32,
	pub(crate) channels: u8,
	pub(crate) channel_mask: ChannelMask,
	pub(crate) version: u8,
	pub(crate) input_sample_rate: u32,
}

impl From<OpusProperties> for FileProperties {
	fn from(input: OpusProperties) -> Self {
		Self {
			duration: input.duration,
			overall_bitrate: Some(input.overall_bitrate),
			audio_bitrate: Some(input.audio_bitrate),
			sample_rate: Some(input.input_sample_rate),
			bit_depth: None,
			channels: Some(input.channels),
			channel_mask: if input.channel_mask == ChannelMask(0) {
				None
			} else {
				Some(input.channel_mask)
			},
		}
	}
}

impl OpusProperties {
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

	/// Channel count
	pub fn channels(&self) -> u8 {
		self.channels
	}

	/// Channel mask
	pub fn channel_mask(&self) -> ChannelMask {
		self.channel_mask
	}

	/// Opus version
	pub fn version(&self) -> u8 {
		self.version
	}

	/// Input sample rate
	pub fn input_sample_rate(&self) -> u32 {
		self.input_sample_rate
	}
}

pub(in crate::ogg) fn read_properties<R>(
	data: &mut R,
	first_page_header: &PageHeader,
	packets: &Packets,
) -> Result<OpusProperties>
where
	R: Read + Seek,
{
	let mut properties = OpusProperties::default();

	// Safe to unwrap, it is impossible to get this far without
	// an identification packet.
	let identification_packet = packets.get(0).unwrap();

	// Skip identification header
	let identification_packet_reader = &mut &identification_packet[8..];

	properties.version = identification_packet_reader.read_u8()?;
	properties.channels = identification_packet_reader.read_u8()?;

	let pre_skip = identification_packet_reader.read_u16::<LittleEndian>()?;

	properties.input_sample_rate = identification_packet_reader.read_u32::<LittleEndian>()?;

	let _output_gain = identification_packet_reader.read_u16::<LittleEndian>()?;

	let channel_mapping_family = identification_packet_reader.read_u8()?;

	// https://datatracker.ietf.org/doc/html/rfc7845.html#section-5.1.1
	if (channel_mapping_family == 0 && properties.channels > 2)
		|| (channel_mapping_family == 1 && properties.channels > 8)
	{
		decode_err!(@BAIL Opus, "Invalid channel count for mapping family");
	}

	properties.channel_mask =
		ChannelMask::from_opus_channels(properties.channels).expect("Channel count is valid");

	let last_page = find_last_page(data);
	let file_length = data.seek(SeekFrom::End(0))?;

	if let Ok(last_page) = last_page {
		let first_page_abgp = first_page_header.abgp;
		let last_page_abgp = last_page.header().abgp;

		let total_samples = last_page_abgp
			.saturating_sub(first_page_abgp)
			// https://datatracker.ietf.org/doc/html/draft-terriberry-oggopus-01#section-4.1:
			//
			// A 'pre-skip' field in the ID header (see Section 5.1) signals the
			// number of samples which should be skipped (decoded but discarded)
			.saturating_sub(u64::from(pre_skip));
		if total_samples > 0 {
			// Best case scenario
			let length = (total_samples * 1000).div_round(48000);

			// Get the stream length by subtracting the length of the header packets

			// Safe to unwrap, metadata is checked prior
			let metadata_packet = packets.get(1).unwrap();
			let header_size = identification_packet.len() + metadata_packet.len();

			let stream_len = file_length - header_size as u64;

			properties.duration = Duration::from_millis(length);
			properties.overall_bitrate = ((file_length * 8) / length) as u32;
			properties.audio_bitrate = ((stream_len * 8) / length) as u32;
		} else {
			log::warn!("Opus: The file contains invalid PCM values, unable to calculate length");
		}
	}

	Ok(properties)
}
