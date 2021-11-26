use super::block::Block;
use super::FlacFile;
use crate::error::{LoftyError, Result};
use crate::logic::ogg::read::read_comments;
use crate::logic::ogg::tag::VorbisComments;
use crate::types::picture::Picture;
use crate::types::properties::FileProperties;

use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

use byteorder::{BigEndian, ReadBytesExt};

pub(in crate::logic::ogg) fn verify_flac<R>(data: &mut R) -> Result<Block>
where
	R: Read + Seek,
{
	let mut marker = [0; 4];
	data.read_exact(&mut marker)?;

	if &marker != b"fLaC" {
		return Err(LoftyError::Flac("File missing \"fLaC\" stream marker"));
	}

	let block = Block::read(data)?;

	if block.ty != 0 {
		return Err(LoftyError::Flac("File missing mandatory STREAMINFO block"));
	}

	Ok(block)
}

fn read_properties<R>(
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
	let channels = ((info >> 9) & 7) + 1;

	// Read the remaining 32 bits of the total samples
	let total_samples = stream_info.read_u32::<BigEndian>()? | (info << 28);

	let (duration, overall_bitrate, audio_bitrate) = if sample_rate > 0 && total_samples > 0 {
		let length = (u64::from(total_samples) * 1000) / u64::from(sample_rate);

		(
			Duration::from_millis(length),
			Some(((file_length * 8) / length) as u32),
			Some(((stream_length * 8) / length) as u32),
		)
	} else {
		(Duration::ZERO, None, None)
	};

	Ok(FileProperties {
		duration,
		overall_bitrate,
		audio_bitrate,
		sample_rate: Some(sample_rate as u32),
		channels: Some(channels as u8),
	})
}

pub(in crate::logic::ogg) fn read_from<R>(data: &mut R) -> Result<FlacFile>
where
	R: Read + Seek,
{
	let stream_info = verify_flac(data)?;
	let stream_info_len = (stream_info.end - stream_info.start) as u32;

	if stream_info_len < 18 {
		return Err(LoftyError::Flac(
			"File has an invalid STREAMINFO block size (< 18)",
		));
	}

	let mut last_block = stream_info.last;

	let mut tag = VorbisComments {
		vendor: String::new(),
		items: vec![],
		pictures: vec![],
	};

	while !last_block {
		let block = Block::read(data)?;
		last_block = block.last;

		match block.ty {
			4 => read_comments(&mut &*block.content, &mut tag)?,
			6 => tag
				.pictures
				.push(Picture::from_flac_bytes(&*block.content)?),
			_ => {},
		}
	}

	let (stream_length, file_length) = {
		let current = data.seek(SeekFrom::Current(0))?;
		let end = data.seek(SeekFrom::End(0))?;

		(end - current, end)
	};

	let properties = read_properties(&mut &*stream_info.content, stream_length, file_length)?;

	Ok(FlacFile {
		properties,
		vorbis_comments: (!(tag.items.is_empty() && tag.pictures.is_empty())).then(|| tag),
	})
}
