use crate::error::Result;
use crate::macros::decode_err;
use crate::properties::FileProperties;

use std::io::Read;
use std::time::Duration;

use byteorder::{BigEndian, ReadBytesExt};

pub(super) fn read_properties(
	comm: &mut &[u8],
	stream_len: u32,
	file_length: u64,
) -> Result<FileProperties> {
	let channels = comm.read_u16::<BigEndian>()? as u8;

	if channels == 0 {
		decode_err!(@BAIL AIFF, "File contains 0 channels");
	}

	let sample_frames = comm.read_u32::<BigEndian>()?;
	let sample_size = comm.read_u16::<BigEndian>()?;

	let mut sample_rate_bytes = [0; 10];
	comm.read_exact(&mut sample_rate_bytes)?;

	let sign = u64::from(sample_rate_bytes[0] & 0x80);

	sample_rate_bytes[0] &= 0x7F;

	let mut exponent = u16::from(sample_rate_bytes[0]) << 8 | u16::from(sample_rate_bytes[1]);
	exponent = exponent - 16383 + 1023;

	let fraction = &mut sample_rate_bytes[2..];
	fraction[0] &= 0x7F;

	let fraction: Vec<u64> = fraction.iter_mut().map(|v| u64::from(*v)).collect();

	let fraction = fraction[0] << 56
		| fraction[1] << 48
		| fraction[2] << 40
		| fraction[3] << 32
		| fraction[4] << 24
		| fraction[5] << 16
		| fraction[6] << 8
		| fraction[7];

	let f64_bytes = sign << 56 | u64::from(exponent) << 52 | fraction >> 11;
	let float = f64::from_be_bytes(f64_bytes.to_be_bytes());

	let sample_rate = float.round() as u32;

	let (duration, overall_bitrate, audio_bitrate) = if sample_rate > 0 && sample_frames > 0 {
		let length = (f64::from(sample_frames) * 1000.0) / f64::from(sample_rate);

		(
			Duration::from_millis(length as u64),
			Some(((file_length as f64) * 8.0 / length + 0.5) as u32),
			Some((f64::from(stream_len) * 8.0 / length + 0.5) as u32),
		)
	} else {
		(Duration::ZERO, None, None)
	};

	Ok(FileProperties {
		duration,
		overall_bitrate,
		audio_bitrate,
		sample_rate: Some(sample_rate),
		bit_depth: Some(sample_size as u8),
		channels: Some(channels),
		channel_mask: None,
	})
}
