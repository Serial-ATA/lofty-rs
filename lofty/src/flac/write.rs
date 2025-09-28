use super::block::{BLOCK_ID_PADDING, BLOCK_ID_PICTURE, BLOCK_ID_VORBIS_COMMENTS, Block};
use super::read::verify_flac;
use crate::config::WriteOptions;
use crate::error::{LoftyError, Result};
use crate::macros::{err, try_vec};
use crate::ogg::tag::VorbisCommentsRef;
use crate::ogg::write::create_comments;
use crate::picture::{Picture, PictureInformation};
use crate::tag::{Tag, TagType};
use crate::util::io::{FileLike, Length, Truncate};

use std::borrow::Cow;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

const BLOCK_HEADER_SIZE: usize = 4;
const MAX_BLOCK_SIZE: u32 = 16_777_215;

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
		TagType::Id3v2 => crate::id3::v2::tag::Id3v2TagRef::empty().write_to(file, write_options),
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
	let stream_info = verify_flac(file)?;

	let mut last_block = stream_info.last;

	let mut file_bytes = Vec::new();
	file.read_to_end(&mut file_bytes)?;

	let mut cursor = Cursor::new(file_bytes);

	// TODO: We need to actually use padding (https://github.com/Serial-ATA/lofty-rs/issues/445)
	let mut end_padding_exists = false;
	let mut last_block_info = (
		stream_info.byte,
		stream_info.start as usize,
		stream_info.end as usize,
	);

	let mut blocks_to_remove = Vec::new();

	while !last_block {
		let block = Block::read(&mut cursor, |block_ty| block_ty == BLOCK_ID_VORBIS_COMMENTS)?;
		let start = block.start;
		let end = block.end;

		let block_type = block.ty;
		last_block = block.last;

		if last_block {
			// (header_first_byte_value, header_start_offset, block_end_offset)
			last_block_info = (block.byte, start as usize, end as usize);
		}

		match block_type {
			BLOCK_ID_VORBIS_COMMENTS => {
				blocks_to_remove.push((start, end));

				// Retain the original vendor string
				let reader = &mut &block.content[..];

				let vendor_len = reader.read_u32::<LittleEndian>()?;
				let mut vendor = try_vec![0; vendor_len as usize];
				reader.read_exact(&mut vendor)?;

				// TODO: Error on strict?
				let Ok(vendor_str) = String::from_utf8(vendor) else {
					log::warn!("FLAC vendor string is not valid UTF-8, not re-using");
					tag.vendor = Cow::Borrowed("");
					continue;
				};

				tag.vendor = Cow::Owned(vendor_str);
			},
			BLOCK_ID_PICTURE => blocks_to_remove.push((start, end)),
			BLOCK_ID_PADDING => {
				if last_block {
					end_padding_exists = true
				} else {
					blocks_to_remove.push((start, end))
				}
			},
			_ => {},
		}
	}

	let mut file_bytes = cursor.into_inner();

	if !end_padding_exists {
		if let Some(preferred_padding) = write_options.preferred_padding {
			log::warn!("File is missing a PADDING block. Adding one");

			let mut first_byte = 0_u8;
			first_byte |= last_block_info.0 & 0x7F;

			file_bytes[last_block_info.1] = first_byte;

			let block_size = core::cmp::min(preferred_padding, MAX_BLOCK_SIZE);
			let mut padding_block = try_vec![0; BLOCK_HEADER_SIZE + block_size as usize];

			let mut padding_byte = 0;
			padding_byte |= 0x80;
			padding_byte |= 1 & 0x7F;

			padding_block[0] = padding_byte;
			padding_block[1..4].copy_from_slice(&block_size.to_be_bytes()[1..]);

			file_bytes.splice(last_block_info.2..last_block_info.2, padding_block);
		}
	}

	let mut comment_blocks = Cursor::new(Vec::new());

	create_comment_block(&mut comment_blocks, &tag.vendor, &mut tag.items)?;

	let mut comment_blocks = comment_blocks.into_inner();

	create_picture_blocks(&mut comment_blocks, &mut tag.pictures)?;

	let ending_with_padding = end_padding_exists || write_options.preferred_padding.is_some() || write_options.preferred_padding != Some(0);

	if !comment_blocks.is_empty() && !ending_with_padding { {
		// Clear the old "last" flag on the previously-last block header byte
		file_bytes[last_block_info.1] &= 0x7F;

		// Make the final block in our inserted sequence the new "last"
		set_last_flag_on_final_block(&mut comment_blocks);
	}

	if blocks_to_remove.is_empty() {
		// If there is end padding, insert before it; otherwise append at the end.
		let insert_at = if end_padding_exists {
			last_block_info.1 // start of the last block (the terminal padding block)
		} else {
			last_block_info.2 // end of the last (non-padding) block
		};
		file_bytes.splice(insert_at..insert_at, comment_blocks);
	} else {
		blocks_to_remove.sort_unstable();
		blocks_to_remove.reverse();

		let first = blocks_to_remove.pop().unwrap(); // Infallible

		for (s, e) in &blocks_to_remove {
			file_bytes.drain(*s as usize..*e as usize);
		}

		file_bytes.splice(first.0 as usize..first.1 as usize, comment_blocks);
	}

	file.seek(SeekFrom::Start(stream_info.end))?;
	file.truncate(stream_info.end)?;
	file.write_all(&file_bytes)?;

	Ok(())
}

fn create_comment_block(
	writer: &mut Cursor<Vec<u8>>,
	vendor: &str,
	items: &mut dyn Iterator<Item = (&str, &str)>,
) -> Result<()> {
	let mut peek = items.peekable();

	if peek.peek().is_some() {
		let mut byte = 0_u8;
		byte |= 4 & 0x7F;

		writer.write_u8(byte)?;
		writer.write_u32::<LittleEndian>(vendor.len() as u32)?;
		writer.write_all(vendor.as_bytes())?;

		let item_count_pos = writer.stream_position()?;
		let mut count = 0;

		writer.write_u32::<LittleEndian>(count)?;

		create_comments(writer, &mut count, &mut peek)?;

		let len = (writer.get_ref().len() - 1) as u32;

		if len > MAX_BLOCK_SIZE {
			err!(TooMuchData);
		}

		let comment_end = writer.stream_position()?;

		writer.seek(SeekFrom::Start(item_count_pos))?;
		writer.write_u32::<LittleEndian>(count)?;

		writer.seek(SeekFrom::Start(comment_end))?;
		writer
			.get_mut()
			.splice(1..1, len.to_be_bytes()[1..].to_vec());

		// size = block type + vendor length + vendor + item count + items
		log::trace!(
			"Wrote a comment block, size: {}",
			1 + 4 + vendor.len() + 4 + (len as usize)
		);
	}

	Ok(())
}

fn create_picture_blocks(
	writer: &mut Vec<u8>,
	pictures: &mut dyn Iterator<Item = (&Picture, PictureInformation)>,
) -> Result<()> {
	let mut byte = 0_u8;
	byte |= 6 & 0x7F;

	for (pic, info) in pictures {
		writer.write_u8(byte)?;

		let pic_bytes = pic.as_flac_bytes(info, false);
		let pic_len = pic_bytes.len() as u32;

		if pic_len > MAX_BLOCK_SIZE {
			err!(TooMuchData);
		}

		writer.write_all(&pic_len.to_be_bytes()[1..])?;
		writer.write_all(pic_bytes.as_slice())?;

		// size = block type + block length + data
		log::trace!("Wrote a picture block, size: {}", 1 + 3 + pic_len);
	}

	Ok(())
}

fn set_last_flag_on_final_block(buf: &mut [u8]) {
	// Walk blocks in `buf`: [1 byte type/last][3 bytes len_be][len bytes data]
	let mut i = 0usize;
	let mut last_hdr = None;
	while i + 4 <= buf.len() {
		last_hdr = Some(i);
		let len = u32::from_be_bytes([0, buf[i + 1], buf[i + 2], buf[i + 3]]) as usize;
		i = i.saturating_add(4).saturating_add(len);
		if i > buf.len() {
			break;
		}
	}
	if let Some(h) = last_hdr {
		buf[h] |= 0x80; // mark final inserted block as "last"
	}
}
