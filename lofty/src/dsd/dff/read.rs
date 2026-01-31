use super::{DffFile, DffProperties};
use crate::config::ParseOptions;
use crate::dsd::dff::properties::LoudspeakerConfig;
use crate::error::{FileDecodingError, Result};
use crate::file::FileType;
use crate::macros::err;

use std::io::{Read, Seek, SeekFrom};

use byteorder::{BigEndian, ReadBytesExt};

/// A simple DFF chunk header reader
///
/// DFF uses 64-bit chunk sizes unlike RIFF/AIFF which use 32-bit,
/// making it incompatible with the standard `Chunks` infrastructure.
/// This provides a minimal reader for DFF's chunk headers.
struct DffChunks {
	fourcc: [u8; 4],
	size: u64,
}

impl DffChunks {
	fn next<R>(&mut self, reader: &mut R) -> Result<bool>
	where
		R: Read,
	{
		// Try to read the chunk header (4 byte ID + 8 byte size)
		if reader.read_exact(&mut self.fourcc).is_err() {
			return Ok(false);
		}

		self.size = reader.read_u64::<BigEndian>()?;
		Ok(true)
	}

	fn skip<R>(&self, reader: &mut R) -> Result<()>
	where
		R: Seek,
	{
		reader.seek(SeekFrom::Current(self.size as i64))?;
		Ok(())
	}
}

/// Verify and read the FRM8 container header
fn verify_dff<R>(reader: &mut R) -> Result<()>
where
	R: Read,
{
	let mut id = [0; 16];
	reader.read_exact(&mut id)?;

	if &id[..4] != b"FRM8" {
		err!(UnknownFormat);
	}

	// Verify form type (id[12..16] should be "DSD ")
	if &id[12..16] != b"DSD " {
		err!(UnknownFormat);
	}

	log::debug!("File verified to be DFF (DSDIFF)");
	Ok(())
}

/// Read a DFF file from a reader
///
/// # Errors
///
/// Returns an error if the file is not a valid DFF file or if I/O fails
pub fn read_from<R>(reader: &mut R, parse_options: ParseOptions) -> Result<DffFile>
where
	R: Read + Seek,
{
	verify_dff(reader)?;

	let mut sample_rate = 0_u32;
	let mut channels = 0_u8;
	let mut sample_count = 0_u64;
	let mut compression = None;
	let mut loudspeaker_config = None;
	let mut id3v2_tag = None;
	let mut dff_text_tag: Option<super::DffTextChunks> = None;

	let mut chunks = DffChunks {
		fourcc: [0; 4],
		size: 0,
	};

	while chunks.next(reader)? {
		match &chunks.fourcc {
			b"FVER" => {
				// Format version - just skip for now
				chunks.skip(reader)?;
			},
			b"PROP" => {
				// Property chunk - contains FS, CHNL, CMPR, etc.
				parse_prop_chunk(
					reader,
					chunks.size,
					&mut sample_rate,
					&mut channels,
					&mut compression,
					&mut loudspeaker_config,
				)?;
			},
			b"DSD " => {
				// Audio data chunk - extract sample count
				// DSD data is 1-bit samples packed 8 per byte
				// Size includes all channel data
				if channels > 0 {
					// Convert bytes to samples: each byte contains 8 samples
					// Total samples per channel = (total_bytes / channels) * 8
					let bytes_per_channel = chunks.size / u64::from(channels);
					sample_count = bytes_per_channel * 8;
				}
				chunks.skip(reader)?;
			},
			b"ID3 " => {
				// ID3v2 tag
				if parse_options.read_tags {
					id3v2_tag = read_id3_chunk(reader, parse_options).ok();
				} else {
					chunks.skip(reader)?;
				}
			},
			b"DIIN" => {
				// DFF text metadata (Edited Master Information)
				if parse_options.read_tags {
					dff_text_tag = parse_diin_chunk(reader, chunks.size).ok();
				} else {
					chunks.skip(reader)?;
				}
			},
			b"COMT" => {
				// DFF comments
				if parse_options.read_tags {
					if let Ok(mut existing_tag) = parse_comt_chunk(reader, chunks.size) {
						if let Some(ref mut tag) = dff_text_tag {
							tag.comments.append(&mut existing_tag.comments);
						} else {
							dff_text_tag = Some(existing_tag);
						}
					}
				} else {
					chunks.skip(reader)?;
				}
			},
			_ => {
				// Unknown chunk - skip
				log::debug!(
					"Skipping unknown DFF chunk: {:?}",
					std::str::from_utf8(&chunks.fourcc).unwrap_or("???")
				);
				chunks.skip(reader)?;
			},
		}
	}

	if sample_rate == 0 || channels == 0 {
		return Err(
			FileDecodingError::new(FileType::Dff, "Missing required FS or CHNL chunk").into(),
		);
	}

	let properties = DffProperties::new(
		sample_rate,
		channels,
		sample_count,
		compression,
		loudspeaker_config,
	);

	Ok(DffFile {
		dff_text_tag,
		id3v2_tag,
		properties,
	})
}

/// Read an ID3v2 chunk from a DFF file
fn read_id3_chunk<R>(
	reader: &mut R,
	parse_options: ParseOptions,
) -> Result<crate::id3::v2::Id3v2Tag>
where
	R: Read + Seek,
{
	use crate::id3::v2::header::Id3v2Header;
	use crate::id3::v2::read::parse_id3v2;

	let header = Id3v2Header::parse(reader)?;
	let id3v2 = parse_id3v2(reader, header, parse_options)?;

	// Skip over the footer if present
	if id3v2.flags().footer {
		reader.seek(SeekFrom::Current(10))?;
	}

	Ok(id3v2)
}

fn parse_prop_chunk<R>(
	reader: &mut R,
	prop_size: u64,
	sample_rate: &mut u32,
	channels: &mut u8,
	compression: &mut Option<String>,
	loudspeaker_config: &mut Option<LoudspeakerConfig>,
) -> Result<()>
where
	R: Read + Seek,
{
	// Read property type (first 4 bytes of PROP chunk)
	let mut prop_type = [0u8; 4];
	reader.read_exact(&mut prop_type)?;

	if &prop_type != b"SND " {
		// Not a sound property chunk, skip remaining bytes
		reader.seek(SeekFrom::Current((prop_size - 4) as i64))?;
		return Ok(());
	}

	// Track how many bytes we've read (4 for prop_type)
	let mut bytes_read = 4_u64;

	// Parse sub-chunks within PROP using the chunk reader pattern
	let mut chunks = DffChunks {
		fourcc: [0; 4],
		size: 0,
	};

	while bytes_read < prop_size && chunks.next(reader)? {
		bytes_read += 12; // 4 bytes fourcc + 8 bytes size

		match &chunks.fourcc {
			b"FS  " => {
				// Sample rate (4 bytes)
				*sample_rate = reader.read_u32::<BigEndian>()?;
				bytes_read += 4;
			},
			b"CHNL" => {
				// Channel count (2 bytes) + channel IDs
				let num_channels = reader.read_u16::<BigEndian>()?;
				*channels = num_channels as u8;
				// Skip channel IDs
				reader.seek(SeekFrom::Current((chunks.size - 2) as i64))?;
				bytes_read += chunks.size;
			},
			b"CMPR" => {
				// Compression type (4 bytes) + compression name
				let mut cmpr_type = [0u8; 4];
				reader.read_exact(&mut cmpr_type)?;
				*compression = Some(String::from_utf8_lossy(&cmpr_type).to_string());
				// Skip compression name
				reader.seek(SeekFrom::Current((chunks.size - 4) as i64))?;
				bytes_read += chunks.size;
			},
			b"LSCO" => {
				// Loudspeaker configuration (2 bytes)
				let config_value = reader.read_u16::<BigEndian>()?;
				*loudspeaker_config = Some(LoudspeakerConfig::from_u16(config_value));
				// Skip remaining bytes if any
				if chunks.size > 2 {
					reader.seek(SeekFrom::Current((chunks.size - 2) as i64))?;
				}
				bytes_read += chunks.size;
			},
			_ => {
				// Unknown sub-chunk - skip
				chunks.skip(reader)?;
				bytes_read += chunks.size;
			},
		}

		if bytes_read >= prop_size {
			break;
		}
	}

	Ok(())
}

fn parse_comt_chunk<R>(reader: &mut R, comt_size: u64) -> Result<super::DffTextChunks>
where
	R: Read + Seek,
{
	use super::DffTextChunks;
	use super::tag::DffComment;

	let mut comments = Vec::new();
	let mut bytes_read = 0_u64;

	// Read number of comments (2 bytes)
	if comt_size < 2 {
		return Ok(DffTextChunks {
			diin: None,
			comments,
		});
	}

	let num_comments = reader.read_u16::<BigEndian>()?;
	bytes_read += 2;

	// Parse each comment record
	for _ in 0..num_comments {
		if bytes_read >= comt_size {
			break;
		}

		// Skip timestamp fields (6 bytes total)
		// timeStampYear (2), Month (1), Day (1), Hour (1), Minutes (1)
		reader.seek(SeekFrom::Current(6))?;
		bytes_read += 6;

		// Skip cmtType (2 bytes) and cmtRef (2 bytes)
		reader.seek(SeekFrom::Current(4))?;
		bytes_read += 4;

		// Read count (4 bytes) - number of characters
		let count = reader.read_u32::<BigEndian>()?;
		bytes_read += 4;

		if count > 0 && bytes_read + u64::from(count) <= comt_size {
			// Read comment text
			let mut text_bytes = vec![0u8; count as usize];
			reader.read_exact(&mut text_bytes)?;
			bytes_read += u64::from(count);

			// Remove null terminator if present
			if let Some(&0) = text_bytes.last() {
				text_bytes.pop();
			}

			if let Ok(text) = String::from_utf8(text_bytes) {
				comments.push(DffComment { text });
			}
		}
	}

	Ok(DffTextChunks {
		diin: None,
		comments,
	})
}

fn parse_diin_chunk<R>(reader: &mut R, diin_size: u64) -> Result<super::DffTextChunks>
where
	R: Read + Seek,
{
	use super::{DffEditedMasterInfo, DffTextChunks};

	let mut artist = None;
	let mut title = None;

	// Track how many bytes we've read
	let mut bytes_read = 0_u64;

	// Parse sub-chunks within DIIN
	let mut chunks = DffChunks {
		fourcc: [0; 4],
		size: 0,
	};

	while bytes_read < diin_size && chunks.next(reader)? {
		bytes_read += 12; // 4 bytes fourcc + 8 bytes size

		match &chunks.fourcc {
			b"DIAR" => {
				// Artist - null-terminated UTF-8 string
				let mut text_bytes = vec![0u8; chunks.size as usize];
				reader.read_exact(&mut text_bytes)?;

				// Remove null terminator if present
				if let Some(&0) = text_bytes.last() {
					text_bytes.pop();
				}

				artist = String::from_utf8(text_bytes).ok();
				bytes_read += chunks.size;
			},
			b"DITI" => {
				// Title - null-terminated UTF-8 string
				let mut text_bytes = vec![0u8; chunks.size as usize];
				reader.read_exact(&mut text_bytes)?;

				// Remove null terminator if present
				if let Some(&0) = text_bytes.last() {
					text_bytes.pop();
				}

				title = String::from_utf8(text_bytes).ok();
				bytes_read += chunks.size;
			},
			_ => {
				// Unknown sub-chunk - skip
				chunks.skip(reader)?;
				bytes_read += chunks.size;
			},
		}

		if bytes_read >= diin_size {
			break;
		}
	}

	let diin =
		(artist.is_some() || title.is_some()).then_some(DffEditedMasterInfo { artist, title });

	Ok(DffTextChunks {
		diin,
		comments: Vec::new(),
	})
}
