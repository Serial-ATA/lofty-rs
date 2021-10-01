use super::WavFile;
use crate::error::{LoftyError, Result};
use crate::logic::id3::v2::read::parse_id3v2;
use crate::types::properties::FileProperties;
use crate::types::tag::{Tag, TagType};

use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

use byteorder::{LittleEndian, ReadBytesExt};

const PCM: u16 = 0x0001;
const IEEE_FLOAT: u16 = 0x0003;
const EXTENSIBLE: u16 = 0xfffe;

pub(in crate::logic::iff) fn verify_wav<T>(data: &mut T) -> Result<()>
where
	T: Read + Seek,
{
	let mut id = [0; 12];
	data.read_exact(&mut id)?;

	if &id[..4] != b"RIFF" {
		return Err(LoftyError::Wav("WAV file doesn't contain a RIFF chunk"));
	}

	if &id[8..] != b"WAVE" {
		return Err(LoftyError::Wav("Found RIFF file, format is not WAVE"));
	}

	Ok(())
}

fn read_properties(fmt: &mut &[u8], total_samples: u32, stream_len: u32) -> Result<FileProperties> {
	let mut format_tag = fmt.read_u16::<LittleEndian>()?;
	let channels = fmt.read_u16::<LittleEndian>()? as u8;

	if channels == 0 {
		return Err(LoftyError::Wav("File contains 0 channels"));
	}

	let sample_rate = fmt.read_u32::<LittleEndian>()?;
	let bytes_per_second = fmt.read_u32::<LittleEndian>()?;

	// Skip 2 bytes
	// Block align (2)
	let _ = fmt.read_u16::<LittleEndian>()?;

	let bits_per_sample = fmt.read_u16::<LittleEndian>()?;

	if format_tag == EXTENSIBLE {
		if fmt.len() < 40 {
			return Err(LoftyError::Wav(
				"Extensible format identified, invalid \"fmt \" chunk size found (< 40)",
			));
		}

		// Skip 8 bytes
		// cbSize (Size of extra format information) (2)
		// Valid bits per sample (2)
		// Channel mask (4)
		let _ = fmt.read_u64::<LittleEndian>()?;

		format_tag = fmt.read_u16::<LittleEndian>()?;
	}

	let non_pcm = format_tag != PCM && format_tag != IEEE_FLOAT;

	if non_pcm && total_samples == 0 {
		return Err(LoftyError::Wav(
			"Non-PCM format identified, no \"fact\" chunk found",
		));
	}

	let sample_frames = if non_pcm {
		total_samples
	} else if bits_per_sample > 0 {
		stream_len / u32::from(u16::from(channels) * ((bits_per_sample + 7) / 8))
	} else {
		0
	};

	let (duration, bitrate) = if sample_rate > 0 && sample_frames > 0 {
		let length = (u64::from(sample_frames) * 1000) / u64::from(sample_rate);

		(
			Duration::from_millis(length),
			(u64::from(stream_len * 8) / length) as u32,
		)
	} else if bytes_per_second > 0 {
		let length = (u64::from(stream_len) * 1000) / u64::from(bytes_per_second);

		(Duration::from_millis(length), (bytes_per_second * 8) / 1000)
	} else {
		(Duration::ZERO, 0)
	};

	Ok(FileProperties::new(
		duration,
		Some(bitrate),
		Some(sample_rate),
		Some(channels),
	))
}

pub(in crate::logic) fn read_from<R>(data: &mut R) -> Result<WavFile>
where
	R: Read + Seek,
{
	verify_wav(data)?;

	let mut stream_len = 0_u32;
	let mut total_samples = 0_u32;
	let mut fmt = Vec::new();

	let mut riff_info = Tag::new(TagType::RiffInfo);
	let mut id3: Option<Tag> = None;

	let mut fourcc = [0; 4];

	while let (Ok(()), Ok(size)) = (
		data.read_exact(&mut fourcc),
		data.read_u32::<LittleEndian>(),
	) {
		match &fourcc {
			b"fmt " => {
				if fmt.is_empty() {
					let mut value = vec![0; size as usize];
					data.read_exact(&mut value)?;

					fmt = value;
					continue;
				}

				data.seek(SeekFrom::Current(i64::from(size)))?;
			}
			b"fact" => {
				if total_samples == 0 {
					total_samples = data.read_u32::<LittleEndian>()?;
					continue;
				}

				data.seek(SeekFrom::Current(4))?;
			}
			b"data" => {
				if stream_len == 0 {
					stream_len += size
				}

				data.seek(SeekFrom::Current(i64::from(size)))?;
			}
			b"LIST" => {
				let mut list_type = [0; 4];
				data.read_exact(&mut list_type)?;

				if &list_type == b"INFO" {
					let end = data.seek(SeekFrom::Current(0))? + u64::from(size - 4);
					super::tag::read::parse_riff_info(data, end, &mut riff_info)?;
				} else {
					data.seek(SeekFrom::Current(i64::from(size)))?;
				}
			}
			b"ID3 " | b"id3 " => {
				let mut value = vec![0; size as usize];
				data.read_exact(&mut value)?;

				let id3v2 = parse_id3v2(&mut &*value)?;

				// Skip over the footer
				if id3v2.flags().footer {
					data.seek(SeekFrom::Current(10))?;
				}

				id3 = Some(id3v2);
			}
			_ => {
				data.seek(SeekFrom::Current(i64::from(size)))?;
			}
		}
	}

	if fmt.len() < 16 {
		return Err(LoftyError::Wav(
			"File does not contain a valid \"fmt \" chunk",
		));
	}

	if stream_len == 0 {
		return Err(LoftyError::Wav("File does not contain a \"data\" chunk"));
	}

	let properties = read_properties(&mut &*fmt, total_samples, stream_len)?;

	Ok(WavFile {
		properties,
		riff_info: (riff_info.item_count() > 0).then(|| riff_info),
		id3v2: id3,
	})
}
