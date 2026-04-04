use super::{DSF_MAGIC, DsfFile, DsfProperties, FMT_CHUNK_SIZE, FMT_MAGIC, HEADER_CHUNK_SIZE};
use crate::config::ParseOptions;
use crate::error::Result;
use crate::id3::v2::header::Id3v2Header;
use crate::id3::v2::read::parse_id3v2;
use crate::macros::{decode_err, err};
use crate::properties::ChannelMask;

use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

use byteorder::{LittleEndian, ReadBytesExt};

pub(crate) fn read_from<R>(reader: &mut R, parse_options: ParseOptions) -> Result<DsfFile>
where
	R: Read + Seek,
{
	let (file_size, metadata_offset) = read_header(reader)?;

	let properties = if parse_options.read_properties {
		read_format_chunk(reader, file_size)?
	} else {
		// Skip the fmt chunk entirely
		let pos = reader.stream_position()?;
		reader.seek(SeekFrom::Start(pos + FMT_CHUNK_SIZE))?;
		DsfProperties::default()
	};

	// Read ID3v2 tag if the metadata offset is non-zero
	let id3v2_tag = if metadata_offset > 0 && parse_options.read_tags {
		reader.seek(SeekFrom::Start(metadata_offset))?;
		let header = Id3v2Header::parse(reader)?;
		Some(parse_id3v2(reader, header, parse_options)?)
	} else {
		None
	};

	Ok(DsfFile {
		id3v2_tag,
		properties,
	})
}

/// Read the DSD chunk header (28 bytes, little-endian)
///
/// Layout:
///   0..4   : magic "DSD "
///   4..12  : chunk size (must be 28)
///   12..20 : total file size
///   20..28 : metadata offset (0 = no ID3v2 tag)
fn read_header<R: Read>(reader: &mut R) -> Result<(u64, u64)> {
	let mut magic = [0u8; 4];
	reader.read_exact(&mut magic)?;

	if &magic != DSF_MAGIC {
		err!(UnknownFormat);
	}

	let chunk_size = reader.read_u64::<LittleEndian>()?;
	if chunk_size != HEADER_CHUNK_SIZE {
		decode_err!(@BAIL Dsf, "Invalid DSD chunk size");
	}

	let file_size = reader.read_u64::<LittleEndian>()?;
	let metadata_offset = reader.read_u64::<LittleEndian>()?;

	Ok((file_size, metadata_offset))
}

/// Read the format chunk (52 bytes, little-endian) and compute audio properties
///
/// Layout:
///   0..4   : magic "fmt "
///   4..12  : chunk size (must be 52)
///   12..16 : format version (must be 1)
///   16..20 : format ID (0 = DSD raw)
///   20..24 : channel type
///   24..28 : channel count
///   28..32 : sample rate
///   32..36 : bits per sample (1 or 8)
///   36..44 : sample count per channel
///   44..48 : block size per channel
///   48..52 : reserved (zero)
fn read_format_chunk<R: Read>(reader: &mut R, file_size: u64) -> Result<DsfProperties> {
	let mut magic = [0u8; 4];
	reader.read_exact(&mut magic)?;

	if &magic != FMT_MAGIC {
		decode_err!(@BAIL Dsf, "Expected fmt chunk");
	}

	let chunk_size = reader.read_u64::<LittleEndian>()?;
	if chunk_size != FMT_CHUNK_SIZE {
		decode_err!(@BAIL Dsf, "Invalid fmt chunk size");
	}

	let format_version = reader.read_u32::<LittleEndian>()?;
	if format_version != 1 {
		decode_err!(@BAIL Dsf, "Unsupported DSF format version");
	}

	let format_id = reader.read_u32::<LittleEndian>()?;
	if format_id != 0 {
		decode_err!(@BAIL Dsf, "Unsupported DSF format ID, only DSD raw is supported");
	}

	let channel_type = reader.read_u32::<LittleEndian>()?;
	let channel_count = reader.read_u32::<LittleEndian>()?;

	if channel_count == 0 || channel_count > 6 {
		decode_err!(@BAIL Dsf, "Invalid channel count");
	}

	let channel_mask = channel_mask_from_dsf_type(channel_type);

	let sample_rate = reader.read_u32::<LittleEndian>()?;
	let bits_per_sample = reader.read_u32::<LittleEndian>()?;

	if bits_per_sample != 1 && bits_per_sample != 8 {
		decode_err!(@BAIL Dsf, "Invalid bits per sample");
	}

	let sample_count = reader.read_u64::<LittleEndian>()?;

	// block_size_per_channel (4 bytes) + reserved (4 bytes)
	let _block_size = reader.read_u32::<LittleEndian>()?;
	let _reserved = reader.read_u32::<LittleEndian>()?;

	let (duration, overall_bitrate, audio_bitrate) = if sample_rate > 0 && sample_count > 0 {
		let duration_ms = (sample_count as f64 / f64::from(sample_rate)) * 1000.0;
		let duration = Duration::from_millis(duration_ms as u64);

		let audio_bitrate =
			((u64::from(sample_rate) * u64::from(channel_count) + 500) / 1000) as u32;
		let overall_bitrate = if duration_ms > 0.0 {
			((file_size as f64 * 8.0 / duration_ms) + 0.5) as u32
		} else {
			audio_bitrate
		};

		(duration, overall_bitrate, audio_bitrate)
	} else {
		(Duration::ZERO, 0, 0)
	};

	Ok(DsfProperties {
		duration,
		overall_bitrate,
		audio_bitrate,
		sample_rate,
		bits_per_sample: bits_per_sample as u8,
		channels: channel_count as u8,
		channel_mask,
	})
}

/// Map DSF channel type to a channel mask
///
/// DSF spec channel types:
///   1 = Mono
///   2 = Stereo
///   3 = 3 channels (L, R, C)
///   4 = Quad (L, R, Ls, Rs)
///   5 = 4 channels (L, R, C, LFE)
///   6 = 5 channels (L, R, C, Ls, Rs)
///   7 = 5.1 (L, R, C, LFE, Ls, Rs)
fn channel_mask_from_dsf_type(channel_type: u32) -> Option<ChannelMask> {
	match channel_type {
		1 => Some(ChannelMask::mono()),
		2 => Some(ChannelMask::stereo()),
		3 => Some(ChannelMask(
			ChannelMask::FRONT_LEFT.bits()
				| ChannelMask::FRONT_RIGHT.bits()
				| ChannelMask::FRONT_CENTER.bits(),
		)),
		4 => Some(ChannelMask(
			ChannelMask::FRONT_LEFT.bits()
				| ChannelMask::FRONT_RIGHT.bits()
				| ChannelMask::BACK_LEFT.bits()
				| ChannelMask::BACK_RIGHT.bits(),
		)),
		5 => Some(ChannelMask(
			ChannelMask::FRONT_LEFT.bits()
				| ChannelMask::FRONT_RIGHT.bits()
				| ChannelMask::FRONT_CENTER.bits()
				| ChannelMask::LOW_FREQUENCY.bits(),
		)),
		6 => Some(ChannelMask(
			ChannelMask::FRONT_LEFT.bits()
				| ChannelMask::FRONT_RIGHT.bits()
				| ChannelMask::FRONT_CENTER.bits()
				| ChannelMask::BACK_LEFT.bits()
				| ChannelMask::BACK_RIGHT.bits(),
		)),
		7 => Some(ChannelMask(
			ChannelMask::FRONT_LEFT.bits()
				| ChannelMask::FRONT_RIGHT.bits()
				| ChannelMask::FRONT_CENTER.bits()
				| ChannelMask::LOW_FREQUENCY.bits()
				| ChannelMask::BACK_LEFT.bits()
				| ChannelMask::BACK_RIGHT.bits(),
		)),
		_ => {
			log::warn!("Unknown DSF channel type: {}", channel_type);
			None
		},
	}
}
