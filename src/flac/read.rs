use super::block::Block;
use super::FlacFile;
use crate::error::Result;
#[cfg(feature = "id3v2")]
use crate::id3::v2::read::parse_id3v2;
use crate::id3::{find_id3v2, ID3FindResults};
use crate::macros::decode_err;
use crate::ogg::read::read_comments;
use crate::ogg::tag::VorbisComments;
use crate::picture::Picture;
use crate::probe::ParseOptions;
use crate::properties::FileProperties;

use std::io::{Read, Seek, SeekFrom};

pub(super) fn verify_flac<R>(data: &mut R) -> Result<Block>
where
	R: Read + Seek,
{
	let mut marker = [0; 4];
	data.read_exact(&mut marker)?;

	if &marker != b"fLaC" {
		decode_err!(@BAIL FLAC, "File missing \"fLaC\" stream marker");
	}

	let block = Block::read(data)?;

	if block.ty != 0 {
		decode_err!(@BAIL FLAC, "File missing mandatory STREAMINFO block");
	}

	Ok(block)
}

pub(crate) fn read_from<R>(data: &mut R, parse_options: ParseOptions) -> Result<FlacFile>
where
	R: Read + Seek,
{
	let mut flac_file = FlacFile {
		#[cfg(feature = "id3v2")]
		id3v2_tag: None,
		vorbis_comments_tag: None,
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
		decode_err!(@BAIL FLAC, "File has an invalid STREAMINFO block size (< 18)");
	}

	let mut last_block = stream_info.last;

	let mut tag = VorbisComments {
		vendor: String::new(),
		items: vec![],
		pictures: vec![],
	};

	while !last_block {
		let block = Block::read(data)?;
		last_block = block.last;

		if block.content.is_empty() && (block.ty != 1 && block.ty != 3) {
			decode_err!(@BAIL FLAC, "Encountered a zero-sized metadata block");
		}

		match block.ty {
			4 => read_comments(&mut &*block.content, block.content.len() as u64, &mut tag)?,
			6 => tag
				.pictures
				.push(Picture::from_flac_bytes(&block.content, false)?),
			_ => {},
		}
	}

	flac_file.vorbis_comments_tag =
		(!(tag.items.is_empty() && tag.pictures.is_empty())).then_some(tag);

	let (stream_length, file_length) = {
		let current = data.stream_position()?;
		let end = data.seek(SeekFrom::End(0))?;

		(end - current, end)
	};

	flac_file.properties = if parse_options.read_properties {
		super::properties::read_properties(&mut &*stream_info.content, stream_length, file_length)?
	} else {
		FileProperties::default()
	};

	Ok(flac_file)
}
