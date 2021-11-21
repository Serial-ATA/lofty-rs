use super::find_last_page;
use crate::error::{LoftyError, Result};
use crate::types::properties::FileProperties;

use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

use byteorder::{LittleEndian, ReadBytesExt};
use ogg_pager::Page;

/// An Opus file's audio properties
pub struct OpusProperties {
	duration: Duration,
	bitrate: u32,
	channels: u8,
	version: u8,
	input_sample_rate: u32,
}

impl From<OpusProperties> for FileProperties {
	fn from(input: OpusProperties) -> Self {
		Self {
			duration: input.duration,
			bitrate: Some(input.bitrate),
			sample_rate: Some(input.input_sample_rate),
			channels: Some(input.channels),
		}
	}
}

impl OpusProperties {
	/// Duration
	pub fn duration(&self) -> Duration {
		self.duration
	}

	/// Bitrate (kbps)
	pub fn bitrate(&self) -> u32 {
		self.bitrate
	}

	/// Channel count
	pub fn channels(&self) -> u8 {
		self.channels
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

pub(in crate::logic::ogg) fn read_properties<R>(
	data: &mut R,
	first_page: &Page,
) -> Result<OpusProperties>
where
	R: Read + Seek,
{
	let stream_len = {
		let current = data.seek(SeekFrom::Current(0))?;
		let end = data.seek(SeekFrom::End(0))?;
		data.seek(SeekFrom::Start(current))?;

		end - first_page.start
	};

	let first_page_abgp = first_page.abgp;

	// Skip identification header
	let first_page_content = &mut &first_page.content[8..];

	let version = first_page_content.read_u8()?;
	let channels = first_page_content.read_u8()?;
	let pre_skip = first_page_content.read_u16::<LittleEndian>()?;
	let input_sample_rate = first_page_content.read_u32::<LittleEndian>()?;

	// Subtract the identification and metadata packet length from the total
	let audio_size = stream_len - data.seek(SeekFrom::Current(0))?;

	let last_page = find_last_page(data)?;
	let last_page_abgp = last_page.abgp;

	last_page_abgp
		.checked_sub(first_page_abgp + u64::from(pre_skip))
		.map_or_else(
			|| Err(LoftyError::Opus("File contains incorrect PCM values")),
			|frame_count| {
				let length = frame_count * 1000 / 48000;
				let duration = Duration::from_millis(length as u64);
				let bitrate = (audio_size * 8 / length) as u32;

				Ok(OpusProperties {
					duration,
					bitrate,
					channels,
					version,
					input_sample_rate,
				})
			},
		)
}
