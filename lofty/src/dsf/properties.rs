use crate::dsf::error::DsfParseError;
use crate::properties::{ChannelMask, FileProperties};
use crate::util::math::RoundedDivision;

use std::io::Read;
use std::time::Duration;

use byteorder::{LittleEndian, ReadBytesExt};

/// The audio encoding format of a DSF file
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FormatId {
	/// Uncompressed DSD audio
	#[default]
	DsdRaw = 0,
	/// Some other format
	Other(u32),
}

impl From<u32> for FormatId {
	fn from(value: u32) -> Self {
		match value {
			0 => Self::DsdRaw,
			_ => Self::Other(value),
		}
	}
}

/// A DSF file's audio properties
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DsfProperties {
	pub(crate) format_id: FormatId,
	pub(crate) duration: Duration,
	pub(crate) overall_bitrate: u32,
	pub(crate) audio_bitrate: u32,
	pub(crate) sample_rate: u32,
	pub(crate) bit_depth: u8,
	pub(crate) channels: u8,
	pub(crate) channel_mask: ChannelMask,
	pub(crate) sample_count: u64,
	pub(crate) block_size: u32,
}

impl DsfProperties {
	/// The audio format
	pub fn format_id(&self) -> FormatId {
		self.format_id
	}

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

	/// Bits per sample (usually 1 or 8)
	pub fn bit_depth(&self) -> u8 {
		self.bit_depth
	}

	/// Channel count
	pub fn channels(&self) -> u8 {
		self.channels
	}

	/// Channel mask
	pub fn channel_mask(&self) -> ChannelMask {
		self.channel_mask
	}

	/// Number of samples per channel
	pub fn sample_count(&self) -> u64 {
		self.sample_count
	}

	/// Block size per channel
	pub fn block_size(&self) -> u32 {
		self.block_size
	}
}

impl From<DsfProperties> for FileProperties {
	fn from(input: DsfProperties) -> Self {
		FileProperties {
			duration: input.duration,
			overall_bitrate: Some(input.overall_bitrate),
			audio_bitrate: Some(input.audio_bitrate),
			sample_rate: Some(input.sample_rate),
			bit_depth: Some(input.bit_depth),
			channels: Some(input.channels),
			channel_mask: Some(input.channel_mask),
		}
	}
}

pub(super) fn read_properties<R>(
	reader: &mut R,
	total_file_size: u64,
	properties: &mut DsfProperties,
) -> Result<(), DsfParseError>
where
	R: Read,
{
	let (id, size) = super::read::read_chunk(reader)?;
	if id != *b"fmt " {
		return Err(DsfParseError::message(format!(
			"expected fmt chunk, found: {}",
			id.escape_ascii()
		)));
	}

	// The spec defines the `fmt ` chunk size as "Usually 52 bytes". Not sure why it's "usually" and
	// not "always"... but we rely on all 52 bytes being occupied anyway
	if size != 52 {
		return Err(DsfParseError::message(format!(
			"expected fmt chunk size of 52, found: {size}"
		)));
	}

	let version = reader.read_u32::<LittleEndian>()?;
	if version != 1 {
		return Err(DsfParseError::message(format!(
			"unsupported format version: {version}"
		)));
	}

	properties.format_id = FormatId::from(reader.read_u32::<LittleEndian>()?);

	let channel_type = reader.read_u32::<LittleEndian>()?;
	let Some(mask) = ChannelMask::from_dsf_channel_type(channel_type as u8) else {
		return Err(DsfParseError::message(format!(
			"unsupported channel type: {channel_type}"
		)));
	};
	properties.channel_mask = mask;

	let channels = reader.read_u32::<LittleEndian>()?;
	if !(1..=6).contains(&channels) {
		return Err(DsfParseError::message(format!(
			"unsupported channel count: {channels}"
		)));
	}
	properties.channels = channels as u8;

	properties.sample_rate = reader.read_u32::<LittleEndian>()?;
	let bit_depth = reader.read_u32::<LittleEndian>()?;
	if bit_depth > 255 {
		log::warn!("bit depth `{bit_depth}` too large, truncating to 255");
	}
	properties.bit_depth = bit_depth as u8;

	properties.audio_bitrate =
		(u32::from(properties.channels) * properties.sample_rate * u32::from(properties.bit_depth))
			.div_round(1000);

	properties.sample_count = reader.read_u64::<LittleEndian>()?;
	properties.block_size = reader.read_u32::<LittleEndian>()?;
	let _reserved = reader.read_u32::<LittleEndian>()?;

	if properties.sample_rate == 0 || properties.sample_count == 0 {
		log::warn!("unable to calculate duration");
		return Ok(());
	}

	let millis = (properties.sample_count * 1000).div_round(u64::from(properties.sample_rate));
	properties.duration = Duration::from_millis(millis);
	properties.overall_bitrate = total_file_size.saturating_mul(8).div_round(millis) as u32;

	Ok(())
}
