use crate::error::Result;
use crate::macros::decode_err;
use crate::ogg::find_last_page;
use crate::properties::FileProperties;
use crate::util::math::RoundedDivision;

use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

use byteorder::{LittleEndian, ReadBytesExt};
use ogg_pager::{Packets, PageHeader};

/// A Speex file's audio properties
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct SpeexProperties {
	pub(crate) duration: Duration,
	pub(crate) version: u32,
	pub(crate) sample_rate: u32,
	pub(crate) mode: u32,
	pub(crate) channels: u8,
	pub(crate) vbr: bool,
	pub(crate) overall_bitrate: u32,
	pub(crate) audio_bitrate: u32,
	pub(crate) nominal_bitrate: i32,
}

impl From<SpeexProperties> for FileProperties {
	fn from(input: SpeexProperties) -> Self {
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

impl SpeexProperties {
	/// Duration of the audio
	pub fn duration(&self) -> Duration {
		self.duration
	}

	/// Speex version
	pub fn version(&self) -> u32 {
		self.version
	}

	/// Sample rate
	pub fn sample_rate(&self) -> u32 {
		self.sample_rate
	}

	/// Speex encoding mode
	pub fn mode(&self) -> u32 {
		self.mode
	}

	/// Channel count
	pub fn channels(&self) -> u8 {
		self.channels
	}

	/// Whether the file makes use of variable bitrate
	pub fn vbr(&self) -> bool {
		self.vbr
	}

	/// Overall bitrate (kbps)
	pub fn overall_bitrate(&self) -> u32 {
		self.overall_bitrate
	}

	/// Audio bitrate (kbps)
	pub fn audio_bitrate(&self) -> u32 {
		self.audio_bitrate
	}

	/// Audio bitrate (bps)
	pub fn nominal_bitrate(&self) -> i32 {
		self.nominal_bitrate
	}
}

pub(in crate::ogg) fn read_properties<R>(
	data: &mut R,
	first_page_header: &PageHeader,
	packets: &Packets,
) -> Result<SpeexProperties>
where
	R: Read + Seek,
{
	log::debug!("Reading Speex properties");

	// Safe to unwrap, it is impossible to get to this point without an
	// identification header.
	let identification_packet = packets.get(0).unwrap();

	if identification_packet.len() < 80 {
		decode_err!(@BAIL Speex, "Header packet too small");
	}

	let mut properties = SpeexProperties::default();

	// The content we need comes 28 bytes into the packet
	//
	// Skipping:
	// Speex string ("Speex   ", 8)
	// Speex version (20)
	let identification_packet_reader = &mut &identification_packet[28..];

	properties.version = identification_packet_reader.read_u32::<LittleEndian>()?;
	if properties.version > 1 {
		decode_err!(@BAIL Speex, "Unknown Speex stream version");
	}

	// Total size of the speex header
	let _header_size = identification_packet_reader.read_u32::<LittleEndian>()?;

	properties.sample_rate = identification_packet_reader.read_u32::<LittleEndian>()?;
	properties.mode = identification_packet_reader.read_u32::<LittleEndian>()?;

	// Version ID of the bitstream
	let _mode_bitstream_version = identification_packet_reader.read_u32::<LittleEndian>()?;

	let channels = identification_packet_reader.read_u32::<LittleEndian>()?;

	if channels != 1 && channels != 2 {
		decode_err!(@BAIL Speex, "Found invalid channel count, must be mono or stereo");
	}

	properties.channels = channels as u8;
	properties.nominal_bitrate = identification_packet_reader.read_i32::<LittleEndian>()?;

	// The size of the frames in samples
	let _frame_size = identification_packet_reader.read_u32::<LittleEndian>()?;

	properties.vbr = identification_packet_reader.read_u32::<LittleEndian>()? == 1;

	let last_page = find_last_page(data);
	let file_length = data.seek(SeekFrom::End(0))?;

	// The stream length is the entire file minus the two mandatory metadata packets
	let metadata_packets_length = packets.iter().take(2).map(<[u8]>::len).sum::<usize>();
	let stream_length = file_length.saturating_sub(metadata_packets_length as u64);

	// This is used for bitrate calculation, it should be the length in
	// milliseconds, but if we can't determine it then we'll just use 1000.
	let mut length = 1000;
	if let Ok(last_page) = last_page {
		let first_page_abgp = first_page_header.abgp;
		let last_page_abgp = last_page.header().abgp;

		if properties.sample_rate > 0 {
			let total_samples = last_page_abgp.saturating_sub(first_page_abgp);

			// Best case scenario
			if total_samples > 0 {
				length = (total_samples * 1000).div_round(u64::from(properties.sample_rate));
				properties.duration = Duration::from_millis(length);
			} else {
				log::warn!(
					"Speex: The file contains invalid PCM values, unable to calculate length"
				);
			}
		} else {
			log::warn!("Speex: Sample rate = 0, unable to calculate length");
		}
	}

	if properties.nominal_bitrate > 0 {
		properties.overall_bitrate = (file_length.saturating_mul(8) / length) as u32;
		properties.audio_bitrate = (properties.nominal_bitrate as u64 / 1000) as u32;
	} else {
		log::warn!("Nominal bitrate = 0, estimating bitrate from file length");

		properties.overall_bitrate = file_length.saturating_mul(8).div_round(length) as u32;
		properties.audio_bitrate = stream_length.saturating_mul(8).div_round(length) as u32;
	}

	Ok(properties)
}
