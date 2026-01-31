use super::tag::{DffCommentRef, DffEditedMasterInfoRef, DffTextChunksRef};
use super::DffFile;
use crate::config::WriteOptions;
use crate::error::{FileEncodingError, LoftyError, Result};
use crate::file::FileType;
use crate::tag::TagExt;
use crate::util::io::{FileLike, Length, Truncate};

use std::io::SeekFrom;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

/// Write ID3v2 tag bytes to a DFF file
///
/// This is called by the generic ID3v2 write infrastructure.
/// It writes the tag in an ID3 chunk within the DFF structure.
///
/// # Errors
///
/// Returns an error if the file is not a valid DFF file or if I/O fails
pub(crate) fn write_id3v2_to_dff<F>(file: &mut F, id3v2_bytes: &[u8]) -> Result<()>
where
	F: FileLike,
	LoftyError: From<<F as Truncate>::Error>,
	LoftyError: From<<F as Length>::Error>,
{
	// DFF has a more complex chunk-based structure
	// We need to find the ID3 chunk and replace it, or add a new one

	// Read FRM8 header
	file.seek(SeekFrom::Start(0))?;
	let mut magic = [0u8; 4];
	file.read_exact(&mut magic)?;

	if &magic != b"FRM8" {
		return Err(FileEncodingError::new(FileType::Dff, "Expected FRM8 magic").into());
	}

	let frm8_size = file.read_u64::<BigEndian>()?;

	// Read form type
	let mut form_type = [0u8; 4];
	file.read_exact(&mut form_type)?;

	if &form_type != b"DSD " {
		return Err(FileEncodingError::new(FileType::Dff, "Expected DSD form type").into());
	}

	// Find the ID3 chunk
	// Note: The FRM8 size field doesn't include the FRM8 magic and size field itself (12 bytes)
	// but it does include the form type (4 bytes), so header_size for chunk iteration is 16
	let header_size = 16; // Position after FRM8 header and form type
	let frm8_header_bytes = 12; // Just FRM8 magic + size field
	let mut id3_chunk_offset = None;
	let mut id3_chunk_size = 0_u64;
	let mut chunks_before_id3 = Vec::new();

	// FRM8 size doesn't include the FRM8 magic and size field (12 bytes total)
	let frm8_end = 12 + frm8_size;
	let mut pos = file.stream_position()?;

	while pos < frm8_end {
		file.seek(SeekFrom::Start(pos))?;

		let mut chunk_id = [0u8; 4];
		if file.read_exact(&mut chunk_id).is_err() {
			break;
		}

		let chunk_size = file.read_u64::<BigEndian>()?;

		if &chunk_id == b"ID3 " {
			id3_chunk_offset = Some(pos);
			id3_chunk_size = chunk_size;
			break;
		}
		chunks_before_id3.push((pos, chunk_size));

		pos += 4 + 8 + chunk_size;
	}

	// Read all data after ID3 chunk (if any)
	let mut data_after_id3 = Vec::new();
	if let Some(id3_offset) = id3_chunk_offset {
		let after_id3_offset = id3_offset + 4 + 8 + id3_chunk_size;
		if after_id3_offset < frm8_end {
			file.seek(SeekFrom::Start(after_id3_offset))?;
			file.read_to_end(&mut data_after_id3)?;
		}
	} else {
		// No existing ID3 chunk, read everything after last chunk
		if let Some((last_chunk_offset, last_chunk_size)) = chunks_before_id3.last() {
			let after_last_offset = last_chunk_offset + 4 + 8 + last_chunk_size;
			if after_last_offset < frm8_end {
				file.seek(SeekFrom::Start(after_last_offset))?;
				file.read_to_end(&mut data_after_id3)?;
			}
		}
	}

	// Calculate new file size
	let new_frm8_size = if id3v2_bytes.is_empty() {
		// Removing ID3 chunk
		frm8_size - (4 + 8 + id3_chunk_size)
	} else if id3_chunk_offset.is_some() {
		// Replacing existing ID3 chunk
		frm8_size - id3_chunk_size + id3v2_bytes.len() as u64
	} else {
		// Adding new ID3 chunk
		frm8_size + 4 + 8 + id3v2_bytes.len() as u64
	};

	// Determine where to write the ID3 chunk
	let id3_write_offset = if let Some(offset) = id3_chunk_offset {
		offset
	} else if let Some((last_offset, last_size)) = chunks_before_id3.last() {
		last_offset + 4 + 8 + last_size
	} else {
		header_size
	};

	// Write the new structure
	file.seek(SeekFrom::Start(id3_write_offset))?;

	if !id3v2_bytes.is_empty() {
		// Write ID3 chunk
		file.write_all(b"ID3 ")?;
		file.write_u64::<BigEndian>(id3v2_bytes.len() as u64)?;
		file.write_all(id3v2_bytes)?;
	}

	// Write data after ID3
	if !data_after_id3.is_empty() {
		file.write_all(&data_after_id3)?;
	}

	// Truncate file
	let new_file_size = frm8_header_bytes + new_frm8_size;
	file.truncate(new_file_size)?;

	// Update FRM8 size
	file.seek(SeekFrom::Start(4))?;
	file.write_u64::<BigEndian>(new_frm8_size)?;

	Ok(())
}

/// Write DIIN chunk bytes to a DFF file
///
/// This finds and replaces or adds a DIIN chunk in the DFF structure.
///
/// # Errors
///
/// Returns an error if the file is not a valid DFF file or if I/O fails
pub fn write_diin_to_dff<F>(file: &mut F, diin_bytes: &[u8]) -> Result<()>
where
	F: FileLike,
	LoftyError: From<<F as Truncate>::Error>,
	LoftyError: From<<F as Length>::Error>,
{
	// Read FRM8 header
	file.seek(SeekFrom::Start(0))?;
	let mut magic = [0u8; 4];
	file.read_exact(&mut magic)?;

	if &magic != b"FRM8" {
		return Err(FileEncodingError::new(FileType::Dff, "Expected FRM8 magic").into());
	}

	let frm8_size = file.read_u64::<BigEndian>()?;

	// Read form type
	let mut form_type = [0u8; 4];
	file.read_exact(&mut form_type)?;

	if &form_type != b"DSD " {
		return Err(FileEncodingError::new(FileType::Dff, "Expected DSD form type").into());
	}

	// Find the DIIN chunk
	let header_size = 16_u64;
	let frm8_header_bytes = 12_u64;
	let mut diin_chunk_offset = None;
	let mut diin_chunk_size = 0_u64;
	let mut chunks_before_diin = Vec::new();

	let frm8_end = 12 + frm8_size;
	let mut pos = file.stream_position()?;

	while pos < frm8_end {
		file.seek(SeekFrom::Start(pos))?;

		let mut chunk_id = [0u8; 4];
		if file.read_exact(&mut chunk_id).is_err() {
			break;
		}

		let chunk_size = file.read_u64::<BigEndian>()?;

		if &chunk_id == b"DIIN" {
			diin_chunk_offset = Some(pos);
			diin_chunk_size = chunk_size;
			break;
		}
		chunks_before_diin.push((pos, chunk_size));

		pos += 4 + 8 + chunk_size;
	}

	// Read all data after DIIN chunk (if any)
	let mut data_after_diin = Vec::new();
	if let Some(diin_offset) = diin_chunk_offset {
		let after_diin_offset = diin_offset + 4 + 8 + diin_chunk_size;
		if after_diin_offset < frm8_end {
			file.seek(SeekFrom::Start(after_diin_offset))?;
			file.read_to_end(&mut data_after_diin)?;
		}
	} else {
		// No existing DIIN chunk, read everything after last chunk
		if let Some((last_chunk_offset, last_chunk_size)) = chunks_before_diin.last() {
			let after_last_offset = last_chunk_offset + 4 + 8 + last_chunk_size;
			if after_last_offset < frm8_end {
				file.seek(SeekFrom::Start(after_last_offset))?;
				file.read_to_end(&mut data_after_diin)?;
			}
		}
	}

	// Calculate new file size
	let new_frm8_size = if diin_bytes.is_empty() {
		// Removing DIIN chunk
		if diin_chunk_offset.is_some() {
			frm8_size - (4 + 8 + diin_chunk_size)
		} else {
			frm8_size
		}
	} else if diin_chunk_offset.is_some() {
		// Replacing existing DIIN chunk
		frm8_size - diin_chunk_size + diin_bytes.len() as u64
	} else {
		// Adding new DIIN chunk
		frm8_size + 4 + 8 + diin_bytes.len() as u64
	};

	// Determine where to write the DIIN chunk
	let diin_write_offset = if let Some(offset) = diin_chunk_offset {
		offset
	} else if let Some((last_offset, last_size)) = chunks_before_diin.last() {
		last_offset + 4 + 8 + last_size
	} else {
		header_size
	};

	// Write the new structure
	file.seek(SeekFrom::Start(diin_write_offset))?;

	if !diin_bytes.is_empty() {
		// Write DIIN chunk (already includes fourcc and size)
		file.write_all(diin_bytes)?;
	}

	// Write data after DIIN
	if !data_after_diin.is_empty() {
		file.write_all(&data_after_diin)?;
	}

	// Truncate file
	let new_file_size = frm8_header_bytes + new_frm8_size;
	file.truncate(new_file_size)?;

	// Update FRM8 size
	file.seek(SeekFrom::Start(4))?;
	file.write_u64::<BigEndian>(new_frm8_size)?;

	Ok(())
}

/// Write COMT chunk bytes to a DFF file
///
/// This finds and replaces or adds a COMT chunk in the DFF structure.
///
/// # Errors
///
/// Returns an error if the file is not a valid DFF file or if I/O fails
pub fn write_comt_to_dff<F>(file: &mut F, comt_bytes: &[u8]) -> Result<()>
where
	F: FileLike,
	LoftyError: From<<F as Truncate>::Error>,
	LoftyError: From<<F as Length>::Error>,
{
	// Read FRM8 header
	file.seek(SeekFrom::Start(0))?;
	let mut magic = [0u8; 4];
	file.read_exact(&mut magic)?;

	if &magic != b"FRM8" {
		return Err(FileEncodingError::new(FileType::Dff, "Expected FRM8 magic").into());
	}

	let frm8_size = file.read_u64::<BigEndian>()?;

	// Read form type
	let mut form_type = [0u8; 4];
	file.read_exact(&mut form_type)?;

	if &form_type != b"DSD " {
		return Err(FileEncodingError::new(FileType::Dff, "Expected DSD form type").into());
	}

	// Find the COMT chunk
	let header_size = 16_u64;
	let frm8_header_bytes = 12_u64;
	let mut comt_chunk_offset = None;
	let mut comt_chunk_size = 0_u64;
	let mut chunks_before_comt = Vec::new();

	let frm8_end = 12 + frm8_size;
	let mut pos = file.stream_position()?;

	while pos < frm8_end {
		file.seek(SeekFrom::Start(pos))?;

		let mut chunk_id = [0u8; 4];
		if file.read_exact(&mut chunk_id).is_err() {
			break;
		}

		let chunk_size = file.read_u64::<BigEndian>()?;

		if &chunk_id == b"COMT" {
			comt_chunk_offset = Some(pos);
			comt_chunk_size = chunk_size;
			break;
		}
		chunks_before_comt.push((pos, chunk_size));

		pos += 4 + 8 + chunk_size;
	}

	// Read all data after COMT chunk (if any)
	let mut data_after_comt = Vec::new();
	if let Some(comt_offset) = comt_chunk_offset {
		let after_comt_offset = comt_offset + 4 + 8 + comt_chunk_size;
		if after_comt_offset < frm8_end {
			file.seek(SeekFrom::Start(after_comt_offset))?;
			file.read_to_end(&mut data_after_comt)?;
		}
	} else {
		// No existing COMT chunk, read everything after last chunk
		if let Some((last_chunk_offset, last_chunk_size)) = chunks_before_comt.last() {
			let after_last_offset = last_chunk_offset + 4 + 8 + last_chunk_size;
			if after_last_offset < frm8_end {
				file.seek(SeekFrom::Start(after_last_offset))?;
				file.read_to_end(&mut data_after_comt)?;
			}
		}
	}

	// Calculate new file size
	let new_frm8_size = if comt_bytes.is_empty() {
		// Removing COMT chunk
		if comt_chunk_offset.is_some() {
			frm8_size - (4 + 8 + comt_chunk_size)
		} else {
			frm8_size
		}
	} else if comt_chunk_offset.is_some() {
		// Replacing existing COMT chunk
		frm8_size - comt_chunk_size + comt_bytes.len() as u64
	} else {
		// Adding new COMT chunk
		frm8_size + 4 + 8 + comt_bytes.len() as u64
	};

	// Determine where to write the COMT chunk
	let comt_write_offset = if let Some(offset) = comt_chunk_offset {
		offset
	} else if let Some((last_offset, last_size)) = chunks_before_comt.last() {
		last_offset + 4 + 8 + last_size
	} else {
		header_size
	};

	// Write the new structure
	file.seek(SeekFrom::Start(comt_write_offset))?;

	if !comt_bytes.is_empty() {
		// Write COMT chunk (already includes fourcc and size)
		file.write_all(comt_bytes)?;
	}

	// Write data after COMT
	if !data_after_comt.is_empty() {
		file.write_all(&data_after_comt)?;
	}

	// Truncate file
	let new_file_size = frm8_header_bytes + new_frm8_size;
	file.truncate(new_file_size)?;

	// Update FRM8 size
	file.seek(SeekFrom::Start(4))?;
	file.write_u64::<BigEndian>(new_frm8_size)?;

	Ok(())
}

/// Write a tag to a DFF file
///
/// # Errors
///
/// Returns an error if the file is not a valid DFF file or if I/O fails
pub(crate) fn write_to<F>(file: &mut F, tag: &crate::tag::Tag, write_options: WriteOptions) -> Result<()>
where
	F: FileLike,
	LoftyError: From<<F as Truncate>::Error>,
	LoftyError: From<<F as Length>::Error>,
{
	use crate::id3::v2::tag::conversion::{tag_frames, Id3v2TagRef};
	use crate::id3::v2::Id3v2TagFlags;
	use crate::tag::TagType;

	match tag.tag_type() {
		TagType::Id3v2 => {
			Id3v2TagRef {
				flags: Id3v2TagFlags::default(),
				frames: tag_frames(tag).peekable(),
			}
			.write_to(file, write_options)
		},
		TagType::DffText => {
			// Convert Tag to DffTextChunksRef without cloning
			let tag_dff: crate::dsd::dff::DffTextChunks = tag.clone().into();
			let diin_ref = tag_dff.diin.as_ref().map(|d| DffEditedMasterInfoRef {
				artist: d.artist.as_deref(),
				title: d.title.as_deref(),
			});
			let comt_refs = tag_dff.comments.iter().map(|c| DffCommentRef {
				text: &c.text,
			});

			DffTextChunksRef {
				diin: diin_ref,
				comments: comt_refs,
			}
			.write_to(file, write_options)
		},
		_ => crate::macros::err!(UnsupportedTag),
	}
}

/// Write DFF file (update metadata only, preserve audio)
///
/// # Errors
///
/// Returns an error if the file is not a valid DFF file or if I/O fails
pub fn write_dff_file<F>(dff_file: &DffFile, file: &mut F, write_options: WriteOptions) -> Result<()>
where
	F: FileLike,
	LoftyError: From<<F as Truncate>::Error>,
	LoftyError: From<<F as Length>::Error>,
{
	// Write DIIN chunk first
	if let Some(dff_text) = &dff_file.dff_text_tag {
		file.rewind()?;
		dff_text.save_to(file, write_options)?;
	} else {
		// No DFF text tag - remove any existing DIIN chunk
		file.rewind()?;
		write_diin_to_dff(file, &[])?;
	}

	// Write ID3v2 chunk
	if let Some(id3v2_tag) = &dff_file.id3v2_tag {
		file.rewind()?;
		id3v2_tag.save_to(file, write_options)?;
	} else {
		// No tag - remove any existing ID3 chunk
		write_id3v2_to_dff(file, &[])?;
	}

	Ok(())
}
