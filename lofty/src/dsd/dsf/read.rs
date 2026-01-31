use super::{
	DATA_MAGIC, DSF_MAGIC, DsfFile, DsfProperties, FMT_CHUNK_SIZE, FMT_MAGIC, HEADER_SIZE,
};
use crate::config::ParseOptions;
use crate::error::{ErrorKind, FileDecodingError, LoftyError, Result};
use crate::file::FileType;
use crate::id3::v2::header::Id3v2Header;
use crate::id3::v2::read::parse_id3v2;
use crate::properties::ChannelMask;

use std::io::{Read, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};

/// Read DSF file from reader
///
/// # Errors
///
/// Returns an error if the file is not a valid DSF file or if I/O fails
pub(super) fn read_from<R: Read + Seek>(
	reader: &mut R,
	parse_options: ParseOptions,
) -> Result<DsfFile> {
	// Read and validate header
	let (_file_size, metadata_pointer) = read_header(reader)?;

	// Read format chunk
	let properties = read_format_chunk(reader)?;

	// Skip data chunk (we don't need to load audio)
	skip_data_chunk(reader)?;

	// Read ID3v2 tag if present
	let id3v2_tag = if metadata_pointer > 0 {
		reader.seek(SeekFrom::Start(metadata_pointer))?;
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

/// Read DSF header (28 bytes, little-endian)
fn read_header<R: Read>(reader: &mut R) -> Result<(u64, u64)> {
	// Magic number (4 bytes): "DSD "
	let mut magic = [0u8; 4];
	reader.read_exact(&mut magic)?;
	if &magic != DSF_MAGIC {
		return Err(LoftyError::new(ErrorKind::UnknownFormat));
	}

	// Chunk size (8 bytes): should be 28
	let chunk_size = reader.read_u64::<LittleEndian>()?;
	if chunk_size != HEADER_SIZE {
		return Err(FileDecodingError::new(FileType::Dsf, "Invalid DSF header chunk size").into());
	}

	// File size (8 bytes)
	let file_size = reader.read_u64::<LittleEndian>()?;

	// Metadata pointer (8 bytes) - 0 if no metadata
	let metadata_pointer = reader.read_u64::<LittleEndian>()?;

	Ok((file_size, metadata_pointer))
}

/// Read format chunk (52 bytes, little-endian)
fn read_format_chunk<R: Read>(reader: &mut R) -> Result<DsfProperties> {
	// Chunk ID (4 bytes): "fmt "
	let mut magic = [0u8; 4];
	reader.read_exact(&mut magic)?;
	if &magic != FMT_MAGIC {
		return Err(FileDecodingError::new(FileType::Dsf, "Expected fmt chunk").into());
	}

	// Chunk size (8 bytes): should be 52
	let chunk_size = reader.read_u64::<LittleEndian>()?;
	if chunk_size != FMT_CHUNK_SIZE {
		return Err(FileDecodingError::new(FileType::Dsf, "Invalid fmt chunk size").into());
	}

	// Format version (4 bytes): should be 1
	let format_version = reader.read_u32::<LittleEndian>()?;
	if format_version != 1 {
		return Err(FileDecodingError::new(FileType::Dsf, "Unsupported DSF format version").into());
	}

	// Format ID (4 bytes): 0 = DSD Raw
	let format_id = reader.read_u32::<LittleEndian>()?;
	if format_id != 0 {
		return Err(FileDecodingError::new(FileType::Dsf, "Only DSD Raw format supported").into());
	}

	// Channel type (4 bytes): 1=mono, 2=stereo, etc.
	let channel_type = reader.read_u32::<LittleEndian>()?;

	// Channel count (4 bytes)
	let channel_count = reader.read_u32::<LittleEndian>()?;
	if !(1..=6).contains(&channel_count) {
		return Err(FileDecodingError::new(FileType::Dsf, "Invalid channel count").into());
	}

	// Convert channel type to channel mask
	let channel_mask = ChannelMask::from_dsf_channel_type(channel_type);

	// Sampling frequency (4 bytes)
	let sample_rate = reader.read_u32::<LittleEndian>()?;
	if !matches!(sample_rate, 2_822_400 | 5_644_800 | 11_289_600 | 22_579_200) {
		return Err(FileDecodingError::new(FileType::Dsf, "Invalid sample rate").into());
	}

	// Bits per sample (4 bytes): 1 or 8
	let bits_per_sample = reader.read_u32::<LittleEndian>()?;
	if bits_per_sample != 1 && bits_per_sample != 8 {
		return Err(FileDecodingError::new(FileType::Dsf, "Invalid bits per sample").into());
	}

	// Sample count (8 bytes)
	let sample_count = reader.read_u64::<LittleEndian>()?;

	// Block size per channel (4 bytes)
	let _block_size = reader.read_u32::<LittleEndian>()?;

	// Reserved (4 bytes)
	let _reserved = reader.read_u32::<LittleEndian>()?;

	Ok(DsfProperties {
		sample_rate,
		channels: channel_count as u8,
		bits_per_sample: bits_per_sample as u8,
		sample_count,
		channel_mask,
	})
}

/// Skip data chunk
fn skip_data_chunk<R: Read + Seek>(reader: &mut R) -> Result<()> {
	// Chunk ID (4 bytes): "data"
	let mut magic = [0u8; 4];
	reader.read_exact(&mut magic)?;
	if &magic != DATA_MAGIC {
		return Err(FileDecodingError::new(FileType::Dsf, "Expected data chunk").into());
	}

	// Chunk size (8 bytes)
	let chunk_size = reader.read_u64::<LittleEndian>()?;

	// Skip audio data
	reader.seek(SeekFrom::Current(chunk_size as i64))?;

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::io::Cursor;

	#[test]
	fn test_read_header() {
		let data = [
			// Magic: "DSD "
			b'D', b'S', b'D', b' ', // Chunk size: 28
			28, 0, 0, 0, 0, 0, 0, 0, // File size: 1000
			0xE8, 0x03, 0, 0, 0, 0, 0, 0, // Metadata pointer: 0
			0, 0, 0, 0, 0, 0, 0, 0,
		];

		let mut cursor = Cursor::new(&data);
		let (file_size, metadata_pointer) = read_header(&mut cursor).unwrap();

		assert_eq!(file_size, 1000);
		assert_eq!(metadata_pointer, 0);
	}

	// Add more tests for format chunk, etc.
}
