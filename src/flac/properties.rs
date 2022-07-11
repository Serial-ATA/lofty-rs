use crate::error::Result;
use crate::properties::FileProperties;

use std::io::Read;
use std::time::Duration;

use byteorder::{BigEndian, ReadBytesExt};

pub(crate) fn read_properties<R>(
	stream_info: &mut R,
	stream_length: u64,
	file_length: u64,
) -> Result<FileProperties>
where
	R: Read,
{
	// Skip 4 bytes
	// Minimum block size (2)
	// Maximum block size (2)
	stream_info.read_u32::<BigEndian>()?;

	// Skip 6 bytes
	// Minimum frame size (3)
	// Maximum frame size (3)
	stream_info.read_uint::<BigEndian>(6)?;

	// Read 4 bytes
	// Sample rate (20 bits)
	// Number of channels (3 bits)
	// Bits per sample (5 bits)
	// Total samples (first 4 bits)
	let info = stream_info.read_u32::<BigEndian>()?;

	let sample_rate = info >> 12;
	let bits_per_sample = ((info >> 4) & 0b11111) + 1;
	let channels = ((info >> 9) & 7) + 1;

	// Read the remaining 32 bits of the total samples
	let total_samples = stream_info.read_u32::<BigEndian>()? | (info << 28);

	let mut properties = FileProperties {
		sample_rate: Some(sample_rate),
		bit_depth: Some(bits_per_sample as u8),
		channels: Some(channels as u8),
		..FileProperties::default()
	};

	if sample_rate > 0 && total_samples > 0 {
		let length = (u64::from(total_samples) * 1000) / u64::from(sample_rate);
		properties.duration = Duration::from_millis(length);

		if length > 0 && file_length > 0 && stream_length > 0 {
			properties.overall_bitrate = Some(((file_length * 8) / length) as u32);
			properties.audio_bitrate = Some(((stream_length * 8) / length) as u32);
		}
	}

	Ok(properties)
}
