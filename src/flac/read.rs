use super::block::Block;
use super::FlacFile;
use crate::error::{FileDecodingError, Result};
use crate::file::FileType;
use crate::id3::ID3FindResults;
#[cfg(feature = "id3v2")]
use crate::id3::{find_id3v2, v2::read::parse_id3v2};
use crate::properties::FileProperties;
#[cfg(feature = "vorbis_comments")]
use crate::{
	ogg::{read::read_comments, tag::VorbisComments},
	picture::Picture,
};

use std::io::{Read, Seek, SeekFrom};

pub(super) fn verify_flac<R>(data: &mut R) -> Result<Block>
where
	R: Read + Seek,
{
	let mut marker = [0; 4];
	data.read_exact(&mut marker)?;

	if &marker != b"fLaC" {
		return Err(
			FileDecodingError::new(FileType::FLAC, "File missing \"fLaC\" stream marker").into(),
		);
	}

	let block = Block::read(data)?;

	if block.ty != 0 {
		return Err(FileDecodingError::new(
			FileType::FLAC,
			"File missing mandatory STREAMINFO block",
		)
		.into());
	}

	Ok(block)
}

pub(crate) fn read_from<R>(data: &mut R, read_properties: bool) -> Result<FlacFile>
where
	R: Read + Seek,
{
	let mut flac_file = FlacFile {
		#[cfg(feature = "id3v2")]
		id3v2_tag: None,
		#[cfg(feature = "vorbis_comments")]
		vorbis_comments: None,
		properties: FileProperties::default(),
	};

	// It is possible for a FLAC file to contain an ID3v2 tag
	if let ID3FindResults(Some(header), Some(content)) = find_id3v2(data, true)? {
		#[cfg(feature = "id3v2")]
		{
			let reader = &mut &*content;

			let id3v2 = parse_id3v2(reader, header)?;
			flac_file.id3v2_tag = Some(id3v2)
		}
	}

	let stream_info = verify_flac(data)?;
	let stream_info_len = (stream_info.end - stream_info.start) as u32;

	if stream_info_len < 18 {
		return Err(FileDecodingError::new(
			FileType::FLAC,
			"File has an invalid STREAMINFO block size (< 18)",
		)
		.into());
	}

	let mut last_block = stream_info.last;

	#[cfg(feature = "vorbis_comments")]
	let mut tag = VorbisComments {
		vendor: String::new(),
		items: vec![],
		pictures: vec![],
	};

	while !last_block {
		let block = Block::read(data)?;
		last_block = block.last;

		match block.ty {
			#[cfg(feature = "vorbis_comments")]
			4 => read_comments(&mut &*block.content, &mut tag)?,
			#[cfg(feature = "vorbis_comments")]
			6 => tag
				.pictures
				.push(Picture::from_flac_bytes(&*block.content, false)?),
			_ => {},
		}
	}

	#[cfg(feature = "vorbis_comments")]
	{
		flac_file.vorbis_comments =
			(!(tag.items.is_empty() && tag.pictures.is_empty())).then(|| tag);
	}

	let (stream_length, file_length) = {
		let current = data.seek(SeekFrom::Current(0))?;
		let end = data.seek(SeekFrom::End(0))?;

		(end - current, end)
	};

	flac_file.properties = if read_properties {
		super::properties::read_properties(&mut &*stream_info.content, stream_length, file_length)?
	} else {
		FileProperties::default()
	};

	Ok(flac_file)
}
