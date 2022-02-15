use crate::error::{FileDecodingError, Result};
use crate::ogg::find_last_page;
use crate::types::file::FileType;
use crate::types::properties::FileProperties;

use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

use byteorder::{LittleEndian, ReadBytesExt};
use ogg_pager::Page;

#[derive(Debug, Copy, Clone, PartialEq, Default)]
#[non_exhaustive]
/// A Speex file's audio properties
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
		}
	}
}

impl SpeexProperties {
	/// Duration
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

pub(in crate::ogg) fn read_properties<R>(data: &mut R, first_page: &Page) -> Result<SpeexProperties>
where
	R: Read + Seek,
{
	let first_page_abgp = first_page.abgp;

	if first_page.content().len() < 80 {
		return Err(FileDecodingError::new(FileType::Speex, "Header packet too small").into());
	}

	let mut properties = SpeexProperties::default();

	// The content we need comes 28 bytes into the packet
	//
	// Skipping:
	// Speex string ("Speex   ", 8)
	// Speex version (20)
	let first_page_content = &mut &first_page.content()[28..];

	properties.version = first_page_content.read_u32::<LittleEndian>()?;

	// Total size of the speex header
	let _header_size = first_page_content.read_u32::<LittleEndian>()?;

	properties.sample_rate = first_page_content.read_u32::<LittleEndian>()?;
	properties.mode = first_page_content.read_u32::<LittleEndian>()?;

	// Version ID of the bitstream
	let _mode_bitstream_version = first_page_content.read_u32::<LittleEndian>()?;

	let channels = first_page_content.read_u32::<LittleEndian>()?;

	if channels != 1 && channels != 2 {
		return Err(FileDecodingError::new(
			FileType::Speex,
			"Found invalid channel count, must be mono or stereo",
		)
		.into());
	}

	properties.channels = channels as u8;
	properties.nominal_bitrate = first_page_content.read_i32::<LittleEndian>()?;

	// The size of the frames in samples
	let _frame_size = first_page_content.read_u32::<LittleEndian>()?;

	properties.vbr = first_page_content.read_u32::<LittleEndian>()? == 1;

	let last_page = find_last_page(data)?;
	let last_page_abgp = last_page.abgp;

	let file_length = data.seek(SeekFrom::End(0))?;

	if let Some(frame_count) = last_page_abgp.checked_sub(first_page_abgp) {
		if properties.sample_rate > 0 {
			let length = frame_count * 1000 / u64::from(properties.sample_rate);
			properties.duration = Duration::from_millis(length as u64);

			properties.overall_bitrate = ((file_length * 8) / length) as u32;
			properties.audio_bitrate = (properties.nominal_bitrate as u64 / 1000) as u32;
		}
	}

	Ok(properties)
}
