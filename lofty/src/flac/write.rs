use super::block::{BLOCK_ID_PADDING, BLOCK_ID_PICTURE, BLOCK_ID_VORBIS_COMMENTS, Block};
use super::read::verify_flac;
use crate::config::WriteOptions;
use crate::error::{LoftyError, Result};
use crate::id3::{FindId3v2Config, find_id3v2};
use crate::macros::{err, try_vec};
use crate::ogg::tag::VorbisCommentsRef;
use crate::picture::{Picture, PictureInformation};
use crate::tag::{Tag, TagType};
use crate::util::io::{FileLike, Length, Truncate};

use std::borrow::Cow;
use std::io::{Cursor, Read, SeekFrom};
use std::iter::Peekable;

use byteorder::{LittleEndian, ReadBytesExt};

pub(crate) fn write_to<F>(file: &mut F, tag: &Tag, write_options: WriteOptions) -> Result<()>
where
	F: FileLike,
	LoftyError: From<<F as Truncate>::Error>,
	LoftyError: From<<F as Length>::Error>,
{
	match tag.tag_type() {
		TagType::VorbisComments => {
			let (vendor, items, pictures) = crate::ogg::tag::create_vorbis_comments_ref(tag);

			let mut comments_ref = VorbisCommentsRef {
				vendor: Cow::from(vendor),
				items,
				pictures,
			};

			write_to_inner(file, &mut comments_ref, write_options)
		},
		// This tag can *only* be removed in this format
		TagType::Id3v2 => {
			crate::id3::v2::tag::conversion::Id3v2TagRef::empty().write_to(file, write_options)
		},
		_ => err!(UnsupportedTag),
	}
}

pub(crate) fn write_to_inner<'a, F, II, IP>(
	file: &mut F,
	tag: &mut VorbisCommentsRef<'a, II, IP>,
	write_options: WriteOptions,
) -> Result<()>
where
	F: FileLike,
	LoftyError: From<<F as Truncate>::Error>,
	II: Iterator<Item = (&'a str, &'a str)>,
	IP: Iterator<Item = (&'a Picture, PictureInformation)>,
{
	let mut file_bytes = Vec::new();
	file.read_to_end(&mut file_bytes)?;

	let mut cursor = Cursor::new(file_bytes);

	// We don't actually need the ID3v2 tag, but reading it will seek to the end of it if it exists
	find_id3v2(&mut cursor, FindId3v2Config::NO_READ_TAG)?;

	let mut stream_info = verify_flac(&mut cursor)?;

	let mut is_last_block = stream_info.last;
	let mut has_blocks_to_remove = false;
	let mut has_padding = false;

	stream_info.last = false; // Determined later

	let mut metadata_range = (stream_info.start as usize)..(stream_info.end as usize);
	let mut blocks = vec![stream_info];
	while !is_last_block {
		let mut skip = false;
		let mut block = Block::read(&mut cursor, |ty| match ty {
			BLOCK_ID_PICTURE => {
				has_blocks_to_remove = true;
				skip = true;
				false
			},
			BLOCK_ID_PADDING => {
				has_padding = true;
				true
			},
			_ => true,
		})?;

		// Retain the original vendor string
		if block.ty == BLOCK_ID_VORBIS_COMMENTS {
			let reader = &mut &block.content[..];

			let vendor_len = reader.read_u32::<LittleEndian>()?;
			let mut vendor_raw = try_vec![0; vendor_len as usize];
			reader.read_exact(&mut vendor_raw)?;

			match String::from_utf8(vendor_raw) {
				Ok(vendor_str) => tag.vendor = Cow::Owned(vendor_str),
				// TODO: Error on strict?
				Err(_) => {
					log::warn!("FLAC vendor string is not valid UTF-8, not re-using");
					tag.vendor = Cow::Borrowed("");
				},
			}

			has_blocks_to_remove = true;
			skip = true;
		}

		is_last_block = block.last;
		metadata_range.end = block.end as usize;

		if !skip {
			// Last block determined later
			block.last = false;
			blocks.push(block);
		}
	}

	let mut comments_peek = (&mut tag.items).peekable();
	let mut pictures_peek = (&mut tag.pictures).peekable();

	let has_comments = comments_peek.peek().is_some();
	let has_pictures = pictures_peek.peek().is_some();

	// Attempting to strip an already empty file
	if !has_blocks_to_remove && !has_comments && !has_pictures {
		log::debug!("Nothing to do");
		return Ok(());
	}

	// TODO: We need to actually use padding (https://github.com/Serial-ATA/lofty-rs/issues/445)
	let will_write_padding = !has_padding && write_options.preferred_padding.is_some();
	let mut file_bytes = cursor.into_inner();

	let metadata_blocks = encode_tag(&tag.vendor, comments_peek, pictures_peek)?;

	blocks.extend(metadata_blocks);

	if will_write_padding {
		if let Some(preferred_padding) = write_options.preferred_padding {
			log::warn!("File is missing a PADDING block. Adding one");

			// `PADDING` always goes last
			let mut padding_block = Block::new_padding(preferred_padding as usize)?;
			padding_block.last = true;

			blocks.push(padding_block);
		}
	}

	if let Some(block) = blocks.last_mut() {
		block.last = true
	}

	let mut encoded_metadata = Vec::new();
	for block in blocks {
		block.write_to(&mut encoded_metadata)?;
		log::trace!(
			"Wrote a block (ty: {}, size: {})",
			block.ty,
			block.content.len()
		);
	}

	file_bytes.splice(metadata_range, encoded_metadata);

	file.seek(SeekFrom::Start(0))?;
	file.write_all(&file_bytes)?;

	Ok(())
}

fn encode_tag<'a, II, IP>(
	vendor: &str,
	mut comments_peek: Peekable<&mut II>,
	mut pictures_peek: Peekable<&mut IP>,
) -> Result<Vec<Block>>
where
	II: Iterator<Item = (&'a str, &'a str)>,
	IP: Iterator<Item = (&'a Picture, PictureInformation)>,
{
	let mut metadata_blocks = Vec::new();

	if comments_peek.peek().is_some() {
		metadata_blocks.push(Block::new_comments(vendor, &mut comments_peek)?);
	}

	loop {
		let Some((picture, info)) = pictures_peek.next() else {
			break;
		};

		metadata_blocks.push(Block::new_picture(picture, info)?);
	}

	Ok(metadata_blocks)
}
