use super::properties::WavProperties;
#[cfg(feature = "riff_info_list")]
use super::tag::RIFFInfoList;
use super::WavFile;
use crate::error::Result;
#[cfg(feature = "id3v2")]
use crate::id3::v2::tag::ID3v2Tag;
use crate::iff::chunk::Chunks;
use crate::macros::decode_err;
use crate::probe::ParseOptions;

use std::io::{Read, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};

pub(super) fn verify_wav<T>(data: &mut T) -> Result<()>
where
	T: Read + Seek,
{
	let mut id = [0; 12];
	data.read_exact(&mut id)?;

	if &id[..4] != b"RIFF" {
		decode_err!(@BAIL WAV, "WAV file doesn't contain a RIFF chunk");
	}

	if &id[8..] != b"WAVE" {
		decode_err!(@BAIL WAV, "Found RIFF file, format is not WAVE");
	}

	Ok(())
}

pub(super) fn read_from<R>(data: &mut R, parse_options: ParseOptions) -> Result<WavFile>
where
	R: Read + Seek,
{
	verify_wav(data)?;

	let current_pos = data.stream_position()?;
	let file_len = data.seek(SeekFrom::End(0))?;

	data.seek(SeekFrom::Start(current_pos))?;

	let mut stream_len = 0_u32;
	let mut total_samples = 0_u32;
	let mut fmt = Vec::new();

	#[cfg(feature = "riff_info_list")]
	let mut riff_info = RIFFInfoList::default();
	#[cfg(feature = "id3v2")]
	let mut id3v2_tag: Option<ID3v2Tag> = None;

	let mut chunks = Chunks::<LittleEndian>::new(file_len);

	while chunks.next(data).is_ok() {
		match &chunks.fourcc {
			b"fmt " if parse_options.read_properties => {
				if fmt.is_empty() {
					fmt = chunks.content(data)?;
				} else {
					chunks.skip(data)?;
				}
			},
			b"fact" if parse_options.read_properties => {
				if total_samples == 0 {
					total_samples = data.read_u32::<LittleEndian>()?;
				} else {
					data.seek(SeekFrom::Current(4))?;
				}
			},
			b"data" if parse_options.read_properties => {
				if stream_len == 0 {
					stream_len += chunks.size
				}

				chunks.skip(data)?;
			},
			b"LIST" => {
				let mut list_type = [0; 4];
				data.read_exact(&mut list_type)?;

				match &list_type {
					#[cfg(feature = "riff_info_list")]
					b"INFO" => {
						let end = data.stream_position()? + u64::from(chunks.size - 4);
						super::tag::read::parse_riff_info(data, &mut chunks, end, &mut riff_info)?;
					},
					_ => {
						data.seek(SeekFrom::Current(-4))?;
						chunks.skip(data)?;
					},
				}
			},
			#[cfg(feature = "id3v2")]
			b"ID3 " | b"id3 " => {
				let tag = chunks.id3_chunk(data)?;
				if let Some(existing_tag) = id3v2_tag.as_mut() {
					// https://github.com/Serial-ATA/lofty-rs/issues/87
					// Duplicate tags should have their frames appended to the previous
					for frame in tag.frames {
						existing_tag.insert(frame);
					}
					continue;
				}
				id3v2_tag = Some(tag);
			},
			_ => chunks.skip(data)?,
		}
	}

	let properties = if parse_options.read_properties {
		if fmt.len() < 16 {
			decode_err!(@BAIL WAV, "File does not contain a valid \"fmt \" chunk");
		}

		if stream_len == 0 {
			decode_err!(@BAIL WAV, "File does not contain a \"data\" chunk");
		}

		let file_length = data.stream_position()?;

		super::properties::read_properties(&mut &*fmt, total_samples, stream_len, file_length)?
	} else {
		WavProperties::default()
	};

	Ok(WavFile {
		properties,
		#[cfg(feature = "riff_info_list")]
		riff_info_tag: (!riff_info.items.is_empty()).then_some(riff_info),
		#[cfg(feature = "id3v2")]
		id3v2_tag,
	})
}
