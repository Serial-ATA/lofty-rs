use super::{DsfFile, DsfProperties};
use crate::config::ParseOptions;
use crate::dsf::error::DsfParseError;
use crate::error::{SizeMismatchError, TagParseError};
use crate::id3::v2::read::parse_id3v2;
use crate::id3::{FindId3v2Config, ID3FindResults, find_id3v2};

use std::io::{Read, Seek, SeekFrom};
use std::num::NonZero;

use byteorder::{LittleEndian, ReadBytesExt};

pub(super) struct DsfChunk {
	pub id: [u8; 4],
	pub size: u64,
}

pub(super) fn read_chunk<R>(reader: &mut R) -> Result<DsfChunk, DsfParseError>
where
	R: Read,
{
	// Chunk sizes include themselves and the ID, so: ID (4) + Size (8)
	const MIN_CHUNK_SIZE: u64 = 12;

	let mut id = [0; 4];
	reader.read_exact(&mut id)?;

	let size = reader.read_u64::<LittleEndian>()?;
	if size < MIN_CHUNK_SIZE {
		return Err(SizeMismatchError.into());
	}

	Ok(DsfChunk { id, size })
}

fn verify_dsf<R>(reader: &mut R) -> Result<(), DsfParseError>
where
	R: Read,
{
	let DsfChunk { id, size } = read_chunk(reader)?;
	if id != *b"DSD " {
		return Err(DsfParseError::message("file missing \"DSD \" chunk"));
	}

	if size != 28 {
		return Err(DsfParseError::message(
			"file has invalid \"DSD \" chunk size",
		));
	}

	Ok(())
}

/// The "DSD " header chunk
#[derive(Debug)]
pub(crate) struct DsdChunk {
	pub total_file_size: u64,
	pub metadata_ptr: Option<NonZero<u64>>,
}

impl DsdChunk {
	pub(crate) fn read<R>(reader: &mut R) -> Result<Self, DsfParseError>
	where
		R: Read,
	{
		verify_dsf(reader)?;

		let total_file_size = reader.read_u64::<LittleEndian>()?;
		let metadata_ptr = NonZero::new(reader.read_u64::<LittleEndian>()?);

		Ok(Self {
			total_file_size,
			metadata_ptr,
		})
	}
}

pub(super) fn read_from<R>(
	reader: &mut R,
	parse_options: ParseOptions,
) -> Result<DsfFile, DsfParseError>
where
	R: Read + Seek,
{
	let dsd_chunk = DsdChunk::read(reader)?;

	let mut id3v2_tag = None;
	let mut properties = DsfProperties::default();

	if parse_options.read_properties {
		super::properties::read_properties(reader, dsd_chunk.total_file_size, &mut properties)?;
	}

	if let Some(metadata_ptr) = dsd_chunk.metadata_ptr
		&& parse_options.read_tags
	{
		reader.seek(SeekFrom::Start(metadata_ptr.get()))?;

		if let Some(ID3FindResults {
			header,
			content: Some(content),
			range: _,
		}) = find_id3v2(reader, FindId3v2Config::READ_TAG).map_err(TagParseError::from)?
		{
			let id3v2 =
				parse_id3v2(&mut &*content, header, parse_options).map_err(TagParseError::from)?;
			id3v2_tag = Some(id3v2);
		} else {
			log::warn!("DSF file has an invalid metadata pointer, no tag was found")
		}
	}

	Ok(DsfFile {
		id3v2_tag,
		properties,
	})
}
