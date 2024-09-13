use super::find_last_page;
use crate::error::Result;
use crate::properties::FileProperties;
use crate::util::math::RoundedDivision;

use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

use byteorder::{LittleEndian, ReadBytesExt};
use ogg_pager::{Packets, PageHeader};

/// An OGG Vorbis file's audio properties
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct VorbisProperties {
	pub(crate) duration: Duration,
	pub(crate) overall_bitrate: u32,
	pub(crate) audio_bitrate: u32,
	pub(crate) sample_rate: u32,
	pub(crate) channels: u8,
	pub(crate) version: u32,
	pub(crate) bitrate_maximum: i32,
	pub(crate) bitrate_nominal: i32,
	pub(crate) bitrate_minimum: i32,
}

impl From<VorbisProperties> for FileProperties {
	fn from(input: VorbisProperties) -> Self {
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

impl VorbisProperties {
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

	/// Vorbis version
	pub fn version(&self) -> u32 {
		self.version
	}

	/// Maximum bitrate (bps)
	pub fn bitrate_max(&self) -> i32 {
		self.bitrate_maximum
	}

	/// Nominal bitrate (bps)
	pub fn bitrate_nominal(&self) -> i32 {
		self.bitrate_nominal
	}

	/// Minimum bitrate (bps)
	pub fn bitrate_min(&self) -> i32 {
		self.bitrate_minimum
	}
}

pub(in crate::ogg) fn read_properties<R>(
	data: &mut R,
	first_page_header: &PageHeader,
	packets: &Packets,
) -> Result<VorbisProperties>
where
	R: Read + Seek,
{
	let mut properties = VorbisProperties::default();

	// It's impossible to get this far without the identification packet, safe to unwrap
	let first_packet = packets.get(0).expect("Identification packet expected");

	// Skip identification header
	let first_page_content = &mut &first_packet[7..];

	properties.version = first_page_content.read_u32::<LittleEndian>()?;

	properties.channels = first_page_content.read_u8()?;
	properties.sample_rate = first_page_content.read_u32::<LittleEndian>()?;

	properties.bitrate_maximum = first_page_content.read_i32::<LittleEndian>()?;
	properties.bitrate_nominal = first_page_content.read_i32::<LittleEndian>()?;
	properties.bitrate_minimum = first_page_content.read_i32::<LittleEndian>()?;

	let last_page = find_last_page(data);
	let file_length = data.seek(SeekFrom::End(0))?;

	// This is used for bitrate calculation, it should be the length in
	// milliseconds, but if we can't determine it then we'll just use 1000.
	let mut length = 1000;
	if let Ok(last_page) = last_page {
		let first_page_abgp = first_page_header.abgp;
		let last_page_abgp = last_page.header().abgp;

		if properties.sample_rate > 0 {
			let total_samples = u128::from(last_page_abgp.saturating_sub(first_page_abgp));

			// Best case scenario
			if total_samples > 0 {
				length =
					(total_samples * 1000).div_round(u128::from(properties.sample_rate)) as u64;
				properties.duration = Duration::from_millis(length);
			} else {
				log::warn!(
					"Vorbis: The file contains invalid PCM values, unable to calculate length"
				);
			}
		} else {
			log::warn!("Vorbis: Sample rate = 0, unable to calculate length");
		}
	}

	if length > 0 {
		properties.overall_bitrate = (file_length.saturating_mul(8) / length) as u32;
	}

	if properties.bitrate_nominal > 0 {
		properties.audio_bitrate = (properties.bitrate_nominal as u64 / 1000) as u32;
	}

	Ok(properties)
}
