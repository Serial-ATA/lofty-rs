use super::find_last_page;
use crate::error::{LoftyError, Result};
use crate::types::properties::FileProperties;

use std::io::{Read, Seek};
use std::time::Duration;

use byteorder::{LittleEndian, ReadBytesExt};
use ogg_pager::Page;

/// An OGG Vorbis file's audio properties
pub struct VorbisProperties {
	duration: Duration,
	bitrate: u32,
	sample_rate: u32,
	channels: u8,
	version: u32,
	bitrate_maximum: u32,
	bitrate_nominal: u32,
	bitrate_minimum: u32,
}

impl From<VorbisProperties> for FileProperties {
	fn from(input: VorbisProperties) -> Self {
		Self {
			duration: input.duration,
			bitrate: Some(input.bitrate),
			sample_rate: Some(input.sample_rate),
			channels: Some(input.channels),
		}
	}
}

impl VorbisProperties {
	/// Duration
	pub fn duration(&self) -> Duration {
		self.duration
	}

	/// Bitrate (kbps)
	pub fn bitrate(&self) -> u32 {
		self.bitrate
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

	/// Maximum bitrate
	pub fn bitrate_max(&self) -> u32 {
		self.bitrate_maximum
	}

	/// Nominal bitrate
	pub fn bitrate_nominal(&self) -> u32 {
		self.bitrate_nominal
	}

	/// Minimum bitrate
	pub fn bitrate_min(&self) -> u32 {
		self.bitrate_minimum
	}
}

pub(in crate::logic::ogg) fn read_properties<R>(
	data: &mut R,
	first_page: &Page,
) -> Result<VorbisProperties>
where
	R: Read + Seek,
{
	let first_page_abgp = first_page.abgp;

	// Skip identification header
	let first_page_content = &mut &first_page.content[7..];

	let version = first_page_content.read_u32::<LittleEndian>()?;

	let channels = first_page_content.read_u8()?;
	let sample_rate = first_page_content.read_u32::<LittleEndian>()?;

	let bitrate_maximum = first_page_content.read_u32::<LittleEndian>()?;
	let bitrate_nominal = first_page_content.read_u32::<LittleEndian>()?;
	let bitrate_minimum = first_page_content.read_u32::<LittleEndian>()?;

	let last_page = find_last_page(data)?;
	let last_page_abgp = last_page.abgp;

	last_page_abgp.checked_sub(first_page_abgp).map_or_else(
		|| Err(LoftyError::Vorbis("File contains incorrect PCM values")),
		|frame_count| {
			let length = frame_count * 1000 / u64::from(sample_rate);
			let duration = Duration::from_millis(length as u64);
			let bitrate = bitrate_nominal / 1000;

			Ok(VorbisProperties {
				duration,
				bitrate,
				sample_rate,
				channels,
				version,
				bitrate_maximum,
				bitrate_nominal,
				bitrate_minimum,
			})
		},
	)
}
