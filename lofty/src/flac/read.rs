use super::FlacFile;
use super::block::Block;
use super::properties::FlacProperties;
use crate::config::{ParseOptions, ParsingMode};
use crate::error::Result;
use crate::flac::block::{BLOCK_ID_PICTURE, BLOCK_ID_STREAMINFO, BLOCK_ID_VORBIS_COMMENTS};
use crate::id3::v2::read::parse_id3v2;
use crate::id3::{FindId3v2Config, ID3FindResults, find_id3v2};
use crate::macros::{decode_err, err};
use crate::ogg::read::read_comments;
use crate::picture::Picture;

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

	let block = Block::read(data, |_| true)?;

	if block.ty != BLOCK_ID_STREAMINFO {
		decode_err!(@BAIL Flac, "File missing mandatory STREAMINFO block");
	}

	log::debug!("File verified to be FLAC");
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

	let find_id3v2_config = if parse_options.read_tags {
		FindId3v2Config::READ_TAG
	} else {
		FindId3v2Config::NO_READ_TAG
	};

	// It is possible for a FLAC file to contain an ID3v2 tag
	if let ID3FindResults(Some(header), Some(content)) = find_id3v2(data, find_id3v2_config)? {
		log::warn!("Encountered an ID3v2 tag. This tag cannot be rewritten to the FLAC file!");

		let reader = &mut &*content;

		let id3v2 = parse_id3v2(reader, header, parse_options)?;
		flac_file.id3v2_tag = Some(id3v2);
	}

	let stream_info = verify_flac(data)?;
	let stream_info_len = (stream_info.end - stream_info.start) as u32;

	if stream_info_len < 18 {
		decode_err!(@BAIL Flac, "File has an invalid STREAMINFO block size (< 18)");
	}

	let mut last_block = stream_info.last;

	while !last_block {
		let block = Block::read(data, |block_type| {
			(block_type == BLOCK_ID_VORBIS_COMMENTS && parse_options.read_tags)
				|| (block_type == BLOCK_ID_PICTURE && parse_options.read_cover_art)
		})?;

		last_block = block.last;

		if block.content.is_empty() {
			continue;
		}

		if block.ty == BLOCK_ID_VORBIS_COMMENTS && parse_options.read_tags {
			log::debug!("Encountered a Vorbis Comments block, parsing");

			// NOTE: According to the spec
			//
			// <https://xiph.org/flac/format.html#def_VORBIS_COMMENT>:
			// "There may be only one VORBIS_COMMENT block in a stream."
			//
			// But of course, we can't ever expect any spec compliant inputs, so we just
			// take whatever happens to be the latest block in the stream. This is safe behavior,
			// as when writing to a file with multiple tags, we end up removing all `VORBIS_COMMENT`
			// blocks anyway.
			if flac_file.vorbis_comments_tag.is_some()
				&& parse_options.parsing_mode == ParsingMode::Strict
			{
				decode_err!(@BAIL Flac, "Streams are only allowed one Vorbis Comments block per stream");
			}

			let vorbis_comments = read_comments(
				&mut &*block.content,
				block.content.len() as u64,
				parse_options,
			)?;

			flac_file.vorbis_comments_tag = Some(vorbis_comments);
			continue;
		}

		if block.ty == BLOCK_ID_PICTURE && parse_options.read_cover_art {
			log::debug!("Encountered a FLAC picture block, parsing");

			match Picture::from_flac_bytes(&block.content, false, parse_options.parsing_mode) {
				Ok(picture) => flac_file.pictures.push(picture),
				Err(e) => {
					if parse_options.parsing_mode == ParsingMode::Strict {
						return Err(e);
					}

					log::warn!("Unable to read FLAC picture block, discarding");
					continue;
				},
			}
		}
	}

	if !parse_options.read_properties {
		return Ok(flac_file);
	}

	let (stream_length, file_length) = {
		let current = data.stream_position()?;
		let end = data.seek(SeekFrom::End(0))?;

		// In the event that a block lies about its size, the current position could be
		// completely wrong.
		if current > end {
			err!(SizeMismatch);
		}

		(end - current, end)
	};

	flac_file.properties =
		super::properties::read_properties(&mut &*stream_info.content, stream_length, file_length)?;

	Ok(flac_file)
}
