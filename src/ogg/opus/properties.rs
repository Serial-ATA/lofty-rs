use super::find_last_page;
use crate::error::Result;
use crate::types::properties::FileProperties;

use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

use byteorder::{LittleEndian, ReadBytesExt};
use ogg_pager::Page;

#[derive(Debug, Copy, Clone, PartialEq, Default)]
#[non_exhaustive]
/// An Opus file's audio properties
pub struct OpusProperties {
	pub(crate) duration: Duration,
	pub(crate) overall_bitrate: u32,
	pub(crate) audio_bitrate: u32,
	pub(crate) channels: u8,
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
		}
	}
}

impl OpusProperties {
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

pub(in crate::ogg) fn read_properties<R>(data: &mut R, first_page: &Page) -> Result<OpusProperties>
where
	R: Read + Seek,
{
	let (stream_len, file_length) = {
		let current = data.seek(SeekFrom::Current(0))?;
		let end = data.seek(SeekFrom::End(0))?;
		data.seek(SeekFrom::Start(current))?;

		(end - first_page.start, end)
	};

	let mut properties = OpusProperties::default();

	let first_page_abgp = first_page.abgp;

	// Skip identification header
	let first_page_content = &mut &first_page.content()[8..];

	properties.version = first_page_content.read_u8()?;
	properties.channels = first_page_content.read_u8()?;

	let pre_skip = first_page_content.read_u16::<LittleEndian>()?;

	properties.input_sample_rate = first_page_content.read_u32::<LittleEndian>()?;

	// Subtract the identification and metadata packet length from the total
	let audio_size = stream_len - data.seek(SeekFrom::Current(0))?;

	let last_page = find_last_page(data)?;
	let last_page_abgp = last_page.abgp;

	if let Some(frame_count) = last_page_abgp.checked_sub(first_page_abgp + u64::from(pre_skip)) {
		let length = frame_count * 1000 / 48000;
		properties.duration = Duration::from_millis(length as u64);

		properties.overall_bitrate = ((file_length * 8) / length) as u32;
		properties.audio_bitrate = (audio_size * 8 / length) as u32;
	}

	Ok(properties)
}
