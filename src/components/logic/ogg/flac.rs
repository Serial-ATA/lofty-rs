use super::read::{read_comments, OGGTags};

use crate::{FileProperties, LoftyError, OggFormat, Picture, Result};

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

use byteorder::{BigEndian, ReadBytesExt};
use unicase::UniCase;

fn read_properties<R>(stream_info: &mut R, stream_length: u64) -> Result<FileProperties>
where
	R: Read,
{
	// Skip 4 bytes
	// Minimum block size (2)
	// Maximum block size (2)
	stream_info.read_u16::<BigEndian>()?;

	// Skip 6 bytes
	// Minimum frame size (3)
	// Maximum frame size (3)
	stream_info.read_uint::<BigEndian>(6)?;

	// Read 24 bits
	// Sample rate (20)
	// Number of channels (3)
	// First bit of bits per sample (1)
	let info = stream_info.read_uint::<BigEndian>(3)?;

	let sample_rate = info >> 4;
	let channels = (info & 0x15) + 1;

	// There are still 4 bits remaining of the bits per sample
	// This number isn't used, so just discard it
	let total_samples_first = stream_info.read_u8()? << 4;

	// Read the remaining 32 bits of the total samples
	let total_samples = stream_info.read_u32::<BigEndian>()? | u32::from(total_samples_first);

	let (duration, bitrate) = if sample_rate > 0 && total_samples > 0 {
		let length = (u64::from(total_samples) * 1000) / sample_rate;

		(
			Duration::from_millis(length),
			((stream_length * 8) / length) as u32,
		)
	} else {
		(Duration::ZERO, 0)
	};

	Ok(FileProperties {
		duration,
		bitrate: Some(bitrate),
		sample_rate: Some(sample_rate as u32),
		channels: Some(channels as u8),
	})
}

pub(crate) fn read_from<R>(data: &mut R) -> Result<OGGTags>
where
	R: Read + Seek,
{
	let mut marker = [0; 4];
	data.read_exact(&mut marker)?;

	if &marker != b"fLaC" {
		return Err(LoftyError::InvalidData(
			"FLAC file missing \"fLaC\" stream marker",
		));
	}

	let mut byte = data.read_u8()?;

	if (byte & 0x7f) != 0 {
		return Err(LoftyError::InvalidData(
			"FLAC file missing mandatory STREAMINFO block",
		));
	}

	let mut last_block = (byte & 0x80) != 0;

	let stream_info_len = data.read_uint::<BigEndian>(3)? as u32;

	if stream_info_len < 18 {
		return Err(LoftyError::InvalidData("FLAC file has an invalid STREAMINFO block size (< 18)"))
	}

	let mut stream_info_data = vec![0; stream_info_len as usize];
	data.read_exact(&mut stream_info_data)?;

	let mut vendor = String::new();
	let mut comments = HashMap::<UniCase<String>, String>::new();
	let mut pictures = Vec::<Picture>::new();

	while !last_block {
		byte = data.read_u8()?;
		last_block = (byte & 0x80) != 0;
		let block_type = byte & 0x7f;

		let block_len = data.read_uint::<BigEndian>(3)? as u32;

		match block_type {
			4 => {
				let mut comment_data = vec![0; block_len as usize];
				data.read_exact(&mut comment_data)?;

				vendor = read_comments(&mut &*comment_data, &mut comments, &mut pictures)?
			},
			6 => {
				let mut picture_data = vec![0; block_len as usize];
				data.read_exact(&mut picture_data)?;

				pictures.push(Picture::from_apic_bytes(&*picture_data)?)
			},
			_ => {
				data.seek(SeekFrom::Current(i64::from(block_len)))?;
				continue;
			},
		}
	}

	let stream_length = {
		let current = data.seek(SeekFrom::Current(0))?;
		let end = data.seek(SeekFrom::End(0))?;
		end - current
	};

	let properties = read_properties(&mut &*stream_info_data, stream_length)?;

	Ok((vendor, pictures, comments, properties, OggFormat::Flac))
}
