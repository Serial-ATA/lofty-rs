use crate::config::ParsingMode;
use crate::error::Result;
use crate::macros::decode_err;
use crate::properties::FileProperties;

use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

use byteorder::{LittleEndian, ReadBytesExt};

/// An APE file's audio properties
#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ApeProperties {
	pub(crate) version: u16,
	pub(crate) duration: Duration,
	pub(crate) overall_bitrate: u32,
	pub(crate) audio_bitrate: u32,
	pub(crate) sample_rate: u32,
	pub(crate) bit_depth: u8,
	pub(crate) channels: u8,
}

impl From<ApeProperties> for FileProperties {
	fn from(input: ApeProperties) -> Self {
		Self {
			duration: input.duration,
			overall_bitrate: Some(input.overall_bitrate),
			audio_bitrate: Some(input.audio_bitrate),
			sample_rate: Some(input.sample_rate),
			bit_depth: Some(input.bit_depth),
			channels: Some(input.channels),
			channel_mask: None,
		}
	}
}

impl ApeProperties {
	/// Duration of the audio
	pub fn duration(&self) -> Duration {
		self.duration
	}

	/// Overall bitrate (kbps)
	pub fn overall_bitrate(&self) -> u32 {
		self.overall_bitrate
	}

	/// Audio bitrate (kbps)
	pub fn bitrate(&self) -> u32 {
		self.audio_bitrate
	}

	/// Sample rate (Hz)
	pub fn sample_rate(&self) -> u32 {
		self.sample_rate
	}

	/// Bits per sample
	pub fn bit_depth(&self) -> u8 {
		self.bit_depth
	}

	/// Channel count
	pub fn channels(&self) -> u8 {
		self.channels
	}

	/// APE version
	pub fn version(&self) -> u16 {
		self.version
	}
}

pub(super) fn read_properties<R>(
	data: &mut R,
	stream_len: u64,
	file_length: u64,
	parse_mode: ParsingMode,
) -> Result<ApeProperties>
where
	R: Read + Seek,
{
	let version = data
		.read_u16::<LittleEndian>()
		.map_err(|_| decode_err!(Ape, "Unable to read APE tag version"))?;

	// Property reading differs between versions
	if version >= 3980 {
		properties_gt_3980(data, version, stream_len, file_length, parse_mode)
	} else {
		properties_lt_3980(data, version, stream_len, file_length, parse_mode)
	}
}

fn properties_gt_3980<R>(
	data: &mut R,
	version: u16,
	stream_len: u64,
	file_length: u64,
	parse_mode: ParsingMode,
) -> Result<ApeProperties>
where
	R: Read + Seek,
{
	// First read the file descriptor
	let mut descriptor = [0; 46];
	data.read_exact(&mut descriptor).map_err(|_| {
		decode_err!(
			Ape,
			"Not enough data left in reader to finish file descriptor"
		)
	})?;

	// The only piece of information we need from the file descriptor
	let descriptor_len = u32::from_le_bytes(
		descriptor[2..6].try_into().unwrap(), // Infallible
	);

	// The descriptor should be 52 bytes long (including ['M', 'A', 'C', ' ']
	// Anything extra is unknown, and just gets skipped
	if descriptor_len > 52 {
		data.seek(SeekFrom::Current(i64::from(descriptor_len - 52)))?;
	}

	// Move on to the header
	let mut header = [0; 24];
	data.read_exact(&mut header)
		.map_err(|_| decode_err!(Ape, "Not enough data left in reader to finish MAC header"))?;

	let mut properties = ApeProperties::default();
	properties.version = version;

	// Skip the first 4 bytes of the header
	// Compression type (2)
	// Format flags (2)
	let header_read = &mut &header[4..];

	let blocks_per_frame = header_read.read_u32::<LittleEndian>()?;
	let final_frame_blocks = header_read.read_u32::<LittleEndian>()?;
	let total_frames = header_read.read_u32::<LittleEndian>()?;

	properties.bit_depth = header_read.read_u16::<LittleEndian>()? as u8;
	properties.channels = header_read.read_u16::<LittleEndian>()? as u8;
	properties.sample_rate = header_read.read_u32::<LittleEndian>()?;

	match verify(total_frames, properties.channels) {
		Err(e) if parse_mode == ParsingMode::Strict => return Err(e),
		Err(_) => return Ok(properties),
		_ => {},
	}

	get_duration_bitrate(
		&mut properties,
		file_length,
		total_frames,
		final_frame_blocks,
		blocks_per_frame,
		stream_len,
	);

	Ok(properties)
}

fn properties_lt_3980<R>(
	data: &mut R,
	version: u16,
	stream_len: u64,
	file_length: u64,
	parse_mode: ParsingMode,
) -> Result<ApeProperties>
where
	R: Read,
{
	// Versions < 3980 don't have a descriptor
	let mut header = [0; 26];
	data.read_exact(&mut header)
		.map_err(|_| decode_err!(Ape, "Not enough data left in reader to finish MAC header"))?;

	let mut properties = ApeProperties::default();
	properties.version = version;

	let header_reader = &mut &header[..];

	// https://github.com/fernandotcl/monkeys-audio/blob/5fe956c7e67c13daa80518a4cc7001e9fa185297/src/MACLib/MACLib.h#L74
	let compression_level = header_reader.read_u16::<LittleEndian>()?;
	let format_flags = header_reader.read_u16::<LittleEndian>()?;
	if (format_flags & 0b1) == 1 {
		properties.bit_depth = 8
	} else if (format_flags & 0b1000) == 8 {
		properties.bit_depth = 24
	} else {
		properties.bit_depth = 16
	}

	let blocks_per_frame = match version {
		_ if version >= 3950 => 73728 * 4,
		_ if version >= 3900 || (version >= 3800 && compression_level >= 4000) => 73728,
		_ => 9216,
	};

	properties.channels = header_reader.read_u16::<LittleEndian>()? as u8;
	properties.sample_rate = header_reader.read_u32::<LittleEndian>()?;

	// Skipping 8 bytes
	// WAV header length (4)
	// WAV tail length (4)
	let mut _skip = [0; 8];
	header_reader.read_exact(&mut _skip)?;

	let total_frames = header_reader.read_u32::<LittleEndian>()?;
	let final_frame_blocks = header_reader.read_u32::<LittleEndian>()?;

	match verify(total_frames, properties.channels) {
		Err(e) if parse_mode == ParsingMode::Strict => return Err(e),
		Err(_) => return Ok(properties),
		_ => {},
	}

	get_duration_bitrate(
		&mut properties,
		file_length,
		total_frames,
		final_frame_blocks,
		blocks_per_frame,
		stream_len,
	);

	Ok(properties)
}

/// Verifies the channel count falls within the bounds of the spec, and we have some audio frames to work with.
fn verify(total_frames: u32, channels: u8) -> Result<()> {
	if !(1..=32).contains(&channels) {
		decode_err!(@BAIL Ape, "File has an invalid channel count (must be between 1 and 32 inclusive)");
	}

	if total_frames == 0 {
		decode_err!(@BAIL Ape, "File contains no frames");
	}

	Ok(())
}

fn get_duration_bitrate(
	properties: &mut ApeProperties,
	file_length: u64,
	total_frames: u32,
	final_frame_blocks: u32,
	blocks_per_frame: u32,
	stream_len: u64,
) {
	let mut total_samples = u64::from(final_frame_blocks);

	if total_samples > 1 {
		total_samples += u64::from(blocks_per_frame) * u64::from(total_frames - 1)
	}

	if properties.sample_rate > 0 {
		let length = (total_samples as f64 * 1000.0) / f64::from(properties.sample_rate);

		properties.duration = Duration::from_millis((length + 0.5) as u64);
		properties.audio_bitrate = ((stream_len as f64) * 8.0 / length + 0.5) as u32;
		properties.overall_bitrate = ((file_length as f64) * 8.0 / length + 0.5) as u32;
	}
}
