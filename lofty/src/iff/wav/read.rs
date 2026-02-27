use super::WavFile;
use super::properties::WavProperties;
use super::tag::RiffInfoList;
use crate::config::ParseOptions;
use crate::error::Result;
use crate::id3::v2::tag::Id3v2Tag;
use crate::iff::chunk::Chunks;
use crate::macros::{decode_err, err};

use std::io::{Read, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};

// Verifies that the stream is a WAV file and returns the stream length
pub(crate) fn verify_wav<T>(data: &mut T) -> Result<u32>
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
	Ok(u32::from_le_bytes(id[4..8].try_into().unwrap()))
}

pub(super) fn read_from<R>(data: &mut R, parse_options: ParseOptions) -> Result<WavFile>
where
	R: Read + Seek,
{
	verify_wav(data)?;

	let current_pos = data.stream_position()?;

	// - 12 for the RIFF chunk we already read
	let file_len = data.seek(SeekFrom::End(0))?.saturating_sub(12);

	data.seek(SeekFrom::Start(current_pos))?;

	let mut stream_len = 0_u32;
	let mut total_samples = 0_u32;
	let mut fmt = Vec::new();

	let mut riff_info = RiffInfoList::default();
	let mut id3v2_tag: Option<Id3v2Tag> = None;

	let mut chunks = Chunks::<_, LittleEndian>::new(data, file_len);
	while let Some(mut chunk) = chunks.next(parse_options.parsing_mode)? {
		match &chunk.fourcc {
			b"fmt " if parse_options.read_properties => {
				if fmt.is_empty() {
					fmt = chunk.content()?;
				}
			},
			b"fact" if parse_options.read_properties => {
				if total_samples == 0 {
					total_samples = chunk.read_u32::<LittleEndian>()?;
				}
			},
			b"data" if parse_options.read_properties => {
				if stream_len == 0 {
					stream_len += chunk.size()
				}
			},
			b"LIST" => {
				let mut size = chunk.size();
				if size < 4 {
					decode_err!(@BAIL Wav, "Invalid LIST chunk size");
				}

				let mut list_type = [0; 4];
				chunk.read_exact(&mut list_type)?;

				size -= 4;

				if &list_type != b"INFO" || !parse_options.read_tags {
					continue;
				}

				let end = chunks.stream_position() + u64::from(size);
				if end > file_len {
					err!(SizeMismatch);
				}

				chunks.lock();
				super::tag::read::parse_riff_info(
					&mut chunks,
					&mut riff_info,
					parse_options.parsing_mode,
				)?;
				chunks.unlock()?;
			},
			b"ID3 " | b"id3 " if parse_options.read_tags => {
				let tag = chunk.id3_chunk(parse_options)?;
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
			_ => {},
		}
	}

	let data = chunks.into_inner();
	let properties = if parse_options.read_properties {
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
