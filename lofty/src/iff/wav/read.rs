use super::properties::WavProperties;
use super::tag::RiffInfoList;
use super::WavFile;
use crate::config::ParseOptions;
use crate::error::Result;
use crate::id3::v2::tag::Id3v2Tag;
use crate::iff::chunk::Chunks;
use crate::macros::decode_err;

use std::io::{Read, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};

pub(super) fn verify_wav<T>(data: &mut T) -> Result<()>
where
	T: Read + Seek,
{
	let mut id = [0; 12];
	data.read_exact(&mut id)?;

	if &id[..4] != b"RIFF" {
		decode_err!(@BAIL Wav, "WAV file doesn't contain a RIFF chunk");
	}

	if &id[8..] != b"WAVE" {
		decode_err!(@BAIL Wav, "Found RIFF file, format is not WAVE");
	}

	log::debug!("File verified to be WAV");
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

	let mut riff_info = RiffInfoList::default();
	let mut id3v2_tag: Option<Id3v2Tag> = None;

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
			b"ID3 " | b"id3 " => {
				let tag = chunks.id3_chunk(data, parse_options.parsing_mode)?;
				if let Some(existing_tag) = id3v2_tag.as_mut() {
					log::warn!("Duplicate ID3v2 tag found, appending frames to previous tag");

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
			decode_err!(@BAIL Wav, "File does not contain a valid \"fmt \" chunk");
		}

		if stream_len == 0 {
			decode_err!(@BAIL Wav, "File does not contain a \"data\" chunk");
		}

		let file_length = data.stream_position()?;

		super::properties::read_properties(&mut &*fmt, total_samples, stream_len, file_length)?
	} else {
		WavProperties::default()
	};

	Ok(WavFile {
		properties,
		riff_info_tag: (!riff_info.items.is_empty()).then_some(riff_info),
		id3v2_tag,
	})
}
