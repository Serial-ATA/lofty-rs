use super::{DATA_MAGIC, FMT_CHUNK_SIZE, HEADER_CHUNK_SIZE};
use crate::error::{FileEncodingError, LoftyError, Result};
use crate::file::FileType;
use crate::util::io::{FileLike, Length, Truncate};

use std::io::{Seek, SeekFrom, Write};

/// Write an ID3v2 tag to a DSF file
///
/// The tag is appended after the audio data, and the DSD chunk header
/// is updated with the new file size and metadata offset.
pub(crate) fn write_to<F>(file: &mut F, tag: &[u8]) -> Result<()>
where
	F: FileLike,
	LoftyError: From<<F as Truncate>::Error>,
	LoftyError: From<<F as Length>::Error>,
{
	// Locate the end of audio data by reading the data chunk size
	file.seek(SeekFrom::Start(HEADER_CHUNK_SIZE + FMT_CHUNK_SIZE))?;

	let mut data_magic = [0u8; 4];
	file.read_exact(&mut data_magic)?;
	if &data_magic != DATA_MAGIC {
		return Err(
			FileEncodingError::new(FileType::Dsf, "Expected data chunk when writing").into(),
		);
	}

	let mut size_bytes = [0u8; 8];
	file.read_exact(&mut size_bytes)?;
	let data_chunk_size = u64::from_le_bytes(size_bytes);

	// The data chunk header is 12 bytes (4 magic + 8 size), and data_chunk_size
	// includes those 12 bytes per the DSF spec
	let audio_end = HEADER_CHUNK_SIZE + FMT_CHUNK_SIZE + data_chunk_size;

	file.seek(SeekFrom::Start(audio_end))?;

	if tag.is_empty() {
		// Strip the tag: truncate after audio data, clear metadata offset
		file.truncate(audio_end)?;
		update_header(file, audio_end, 0)?;
	} else {
		// Write tag at the end of audio data
		file.write_all(tag)?;

		let new_file_size = audio_end + tag.len() as u64;
		file.truncate(new_file_size)?;
		update_header(file, new_file_size, audio_end)?;
	}

	Ok(())
}

/// Update the DSD chunk header with new file size and metadata offset
fn update_header<W: Write + Seek>(
	writer: &mut W,
	file_size: u64,
	metadata_offset: u64,
) -> Result<()> {
	// Offset 12..20: total file size
	writer.seek(SeekFrom::Start(12))?;
	writer.write_all(&file_size.to_le_bytes())?;

	// Offset 20..28: metadata offset (0 if no metadata)
	writer.write_all(&metadata_offset.to_le_bytes())?;

	Ok(())
}
