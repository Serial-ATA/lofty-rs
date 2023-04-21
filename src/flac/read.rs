use super::block::Block;
use super::properties::FlacProperties;
use super::FlacFile;
use crate::error::Result;
use crate::flac::block::{
	BLOCK_ID_PADDING, BLOCK_ID_PICTURE, BLOCK_ID_SEEKTABLE, BLOCK_ID_STREAMINFO,
	BLOCK_ID_VORBIS_COMMENTS,
};
use crate::id3::v2::read::parse_id3v2;
use crate::id3::{find_id3v2, ID3FindResults};
use crate::macros::decode_err;
use crate::ogg::read::read_comments;
use crate::ogg::tag::VorbisComments;
use crate::picture::Picture;
use crate::probe::ParseOptions;

use std::io::{Read, Seek, SeekFrom};

pub(super) fn verify_flac<R>(data: &mut R) -> Result<Block>
where
	R: Read + Seek,
{
	let mut marker = [0; 4];
	data.read_exact(&mut marker)?;

	if &marker != b"fLaC" {
		decode_err!(@BAIL Flac, "File missing \"fLaC\" stream marker");
	}

	let block = Block::read(data)?;

	if block.ty != BLOCK_ID_STREAMINFO {
		decode_err!(@BAIL Flac, "File missing mandatory STREAMINFO block");
	}

	Ok(block)
}

pub(crate) fn read_from<R>(data: &mut R, parse_options: ParseOptions) -> Result<FlacFile>
where
	R: Read + Seek,
{
	let mut flac_file = FlacFile {
		id3v2_tag: None,
		vorbis_comments_tag: None,
		pictures: Vec::new(),
		properties: FlacProperties::default(),
	};

	// It is possible for a FLAC file to contain an ID3v2 tag
	if let ID3FindResults(Some(header), Some(content)) = find_id3v2(data, true)? {
		let reader = &mut &*content;

		let id3v2 = parse_id3v2(reader, header)?;
		flac_file.id3v2_tag = Some(id3v2);
	}

	let stream_info = verify_flac(data)?;
	let stream_info_len = (stream_info.end - stream_info.start) as u32;

	if stream_info_len < 18 {
		decode_err!(@BAIL Flac, "File has an invalid STREAMINFO block size (< 18)");
	}

	let mut last_block = stream_info.last;

	while !last_block {
		let block = Block::read(data)?;
		last_block = block.last;

		if block.content.is_empty()
			&& (block.ty != BLOCK_ID_PADDING && block.ty != BLOCK_ID_SEEKTABLE)
		{
			decode_err!(@BAIL Flac, "Encountered a zero-sized metadata block");
		}

		if block.ty == BLOCK_ID_VORBIS_COMMENTS {
			if flac_file.vorbis_comments_tag.is_some() {
				decode_err!(@BAIL Flac, "Streams are only allowed one Vorbis Comments block per stream");
			}

			let mut vorbis_comments = VorbisComments::new();
			read_comments(
				&mut &*block.content,
				block.content.len() as u64,
				&mut vorbis_comments,
			)?;

			flac_file.vorbis_comments_tag = Some(vorbis_comments);
			continue;
		}

		if block.ty == BLOCK_ID_PICTURE {
			flac_file
				.pictures
				.push(Picture::from_flac_bytes(&block.content, false)?)
		}
	}

	if !parse_options.read_properties {
		return Ok(flac_file);
	}

	let (stream_length, file_length) = {
		let current = data.stream_position()?;
		let end = data.seek(SeekFrom::End(0))?;

		(end - current, end)
	};

	flac_file.properties =
		super::properties::read_properties(&mut &*stream_info.content, stream_length, file_length)?;

	Ok(flac_file)
}
