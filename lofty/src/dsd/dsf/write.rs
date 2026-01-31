use super::{DATA_MAGIC, DsfFile, FMT_CHUNK_SIZE, HEADER_SIZE};
use crate::config::WriteOptions;
use crate::error::{FileEncodingError, LoftyError, Result};
use crate::file::FileType;
use crate::tag::TagExt;
use crate::util::io::{FileLike, Length, Truncate};
use std::io::{Seek, SeekFrom, Write};

/// Write ID3v2 tag bytes to a DSF file
///
/// This is called by the generic ID3v2 write infrastructure.
/// It writes the tag at the end of the file and updates the header pointer.
///
/// # Errors
///
/// Returns an error if the file is not a valid DSF file or if I/O fails
pub(crate) fn write_id3v2_to_dsf<F>(file: &mut F, id3v2_bytes: &[u8]) -> Result<()>
where
	F: FileLike,
	LoftyError: From<<F as Truncate>::Error>,
	LoftyError: From<<F as Length>::Error>,
{
	// Find the end of audio data
	file.seek(SeekFrom::Start(HEADER_SIZE + FMT_CHUNK_SIZE))?;

	// Read data chunk header
	let mut data_magic = [0u8; 4];
	file.read_exact(&mut data_magic)?;
	if &data_magic != DATA_MAGIC {
		return Err(
			FileEncodingError::new(FileType::Dsf, "Expected data chunk when writing").into(),
		);
	}

	// Read data chunk size
	let mut size_bytes = [0u8; 8];
	file.read_exact(&mut size_bytes)?;
	let data_chunk_size = u64::from_le_bytes(size_bytes);

	// Calculate end of audio data
	let audio_end_offset = HEADER_SIZE + FMT_CHUNK_SIZE + 12 + data_chunk_size;

	// Write tag at end of audio
	file.seek(SeekFrom::Start(audio_end_offset))?;

	let (new_file_size, metadata_pointer) = if id3v2_bytes.is_empty() {
		// No tag - truncate after audio
		file.truncate(audio_end_offset)?;
		(audio_end_offset, 0)
	} else {
		// Write tag
		file.write_all(id3v2_bytes)?;
		let new_file_size = audio_end_offset + id3v2_bytes.len() as u64;
		file.truncate(new_file_size)?;
		(new_file_size, audio_end_offset)
	};

	// Update header
	update_header(file, new_file_size, metadata_pointer)?;

	Ok(())
}

/// Write a tag to a DSF file
///
/// # Errors
///
/// Returns an error if the file is not a valid DSF file or if I/O fails
pub(crate) fn write_to<F>(
	file: &mut F,
	tag: &crate::tag::Tag,
	write_options: WriteOptions,
) -> Result<()>
where
	F: FileLike,
	LoftyError: From<<F as Truncate>::Error>,
	LoftyError: From<<F as Length>::Error>,
{
	use crate::id3::v2::tag::conversion::{tag_frames, Id3v2TagRef};
	use crate::id3::v2::Id3v2TagFlags;

	Id3v2TagRef {
		flags: Id3v2TagFlags::default(),
		frames: tag_frames(tag).peekable(),
	}
	.write_to(file, write_options)
}

/// Write DSF file (update metadata only, preserve audio)
///
/// This function updates the ID3v2 tag at the end of a DSF file while
/// preserving the audio data. The file must be open for both reading and writing.
///
/// # Errors
///
/// Returns an error if the file is not a valid DSF file or if I/O fails
pub fn write_dsf_file<F>(dsf_file: &DsfFile, file: &mut F, write_options: WriteOptions) -> Result<()>
where
	F: FileLike,
	LoftyError: From<<F as Truncate>::Error>,
	LoftyError: From<<F as Length>::Error>,
{
	// If we have an ID3v2 tag, use the generic tag writing infrastructure
	// which will call back to write_id3v2_to_dsf for DSF-specific handling
	if let Some(id3v2_tag) = &dsf_file.id3v2_tag {
		file.rewind()?;
		id3v2_tag.save_to(file, write_options)?;
	} else {
		// No tag - remove any existing tag by truncating after audio
		// Find the end of audio data
		file.seek(SeekFrom::Start(HEADER_SIZE + FMT_CHUNK_SIZE))?;

		let mut data_magic = [0u8; 4];
		file.read_exact(&mut data_magic)?;
		if &data_magic != DATA_MAGIC {
			return Err(FileEncodingError::new(FileType::Dsf, "Expected data chunk").into());
		}

		let mut size_bytes = [0u8; 8];
		file.read_exact(&mut size_bytes)?;
		let data_chunk_size = u64::from_le_bytes(size_bytes);

		let audio_end_offset = HEADER_SIZE + FMT_CHUNK_SIZE + 12 + data_chunk_size;

		// Truncate and update header
		file.truncate(audio_end_offset)?;
		update_header(file, audio_end_offset, 0)?;
	}

	Ok(())
}

/// Update DSF header with new file size and metadata pointer
fn update_header<W: Write + Seek>(
	writer: &mut W,
	file_size: u64,
	metadata_pointer: u64,
) -> Result<()> {
	// DSF header structure:
	// Offset 0-3: Magic "DSD "
	// Offset 4-11: Chunk size (always 28)
	// Offset 12-19: Total file size
	// Offset 20-27: Metadata pointer (0 if no metadata)

	// Seek to file_size position (offset 12)
	writer.seek(SeekFrom::Start(12))?;
	writer.write_all(&file_size.to_le_bytes())?;

	// Seek to metadata_pointer position (offset 20)
	writer.seek(SeekFrom::Start(20))?;
	writer.write_all(&metadata_pointer.to_le_bytes())?;

	Ok(())
}
