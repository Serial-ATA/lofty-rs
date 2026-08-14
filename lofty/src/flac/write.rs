use super::block::{BLOCK_ID_PADDING, BLOCK_ID_PICTURE, BLOCK_ID_VORBIS_COMMENTS, Block};
use super::read::verify_flac;
use crate::config::WriteOptions;
use crate::error::{FileEncodingError, FileParseError, SizeMismatchError, TagParseError};
use crate::id3::{FindId3v2Config, find_id3v2};
use crate::io::{Length, VerifiedFile};
use crate::macros::try_vec;
use crate::ogg::tag::VorbisCommentsRef;
use crate::picture::{Picture, PictureInformation};
use crate::tag::{Tag, TagType};
use crate::util::io::FileLike;

use std::borrow::Cow;
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use std::iter::Peekable;
use std::ops::Range;

use byteorder::{LittleEndian, ReadBytesExt};

pub(crate) fn write_to<F>(
	file: VerifiedFile<'_, F>,
	tag: &Tag,
	write_options: WriteOptions,
) -> Result<(), FileEncodingError>
where
	F: FileLike,
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
		_ => unreachable!("tag type verified beforehand"),
	}
}

pub(crate) fn write_to_inner<'a, F, II, IP>(
	file: VerifiedFile<'_, F>,
	tag: &mut VorbisCommentsRef<'a, II, IP>,
	write_options: WriteOptions,
) -> Result<(), FileEncodingError>
where
	F: FileLike,
	II: Iterator<Item = (&'a str, &'a str)>,
	IP: Iterator<Item = (&'a Picture, PictureInformation)>,
{
	let mut file = file.into_inner();

	file.rewind()?;

	// We don't actually need the ID3v2 tag, but reading it will seek to the end of it if it exists
	find_id3v2(&mut file, FindId3v2Config::NO_READ_TAG).map_err(TagParseError::from)?;

	let mut stream_info = verify_flac(&mut file).map_err(FileParseError::from)?;

	let mut is_last_block = stream_info.last;
	let mut has_blocks_to_remove = false;
	let mut has_padding = false;

	stream_info.last = false; // Determined later

	let mut metadata_range = stream_info.start..stream_info.end;
	let mut blocks = vec![stream_info];
	while !is_last_block {
		let mut skip = false;
		let mut block = Block::read(&mut file, |ty| match ty {
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
		})
		.map_err(FileParseError::from)?;

		// Retain the original vendor string
		if block.ty == BLOCK_ID_VORBIS_COMMENTS {
			let reader = &mut &block.content[..];

			let vendor_len = reader.read_u32::<LittleEndian>()?;
			if vendor_len as usize > reader.len() {
				return Err(SizeMismatchError.into());
			}

			let mut vendor_raw = try_vec![0; vendor_len as usize]?;
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
		metadata_range.end = block.end;

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

	let metadata_blocks = encode_tag(&tag.vendor, comments_peek, pictures_peek)?;

	blocks.extend(metadata_blocks);

	if will_write_padding && let Some(preferred_padding) = write_options.preferred_padding {
		log::warn!("File is missing a PADDING block. Adding one");

		let metadata_len = blocks
			.iter()
			.map(|block| u64::from(block.len()))
			.sum::<u64>();
		let old_metadata_len = metadata_range.end - metadata_range.start;
		let available_padding = old_metadata_len
			.saturating_sub(metadata_len)
			.saturating_sub(Block::BLOCK_HEADER_SIZE as u64);
		let padding_len = available_padding
			.max(u64::from(preferred_padding))
			.min(u64::from(Block::MAX_CONTENT_SIZE));
		let padding_len = usize::try_from(padding_len).map_err(|_| SizeMismatchError)?;

		// `PADDING` always goes last
		blocks.push(Block::new_padding(padding_len)?);
	}

	if let Some(block) = blocks.last_mut() {
		block.last = true;
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

	replace_range(&mut file, metadata_range, &encoded_metadata)?;

	Ok(())
}

const MOVE_BUFFER_SIZE: usize = 64 * 1024;

fn replace_range<F>(file: &mut F, range: Range<u64>, replacement: &[u8]) -> std::io::Result<()>
where
	F: FileLike,
{
	if range.start > range.end {
		return Err(std::io::Error::new(
			ErrorKind::InvalidInput,
			"range start exceeds range end",
		));
	}

	let file_len = Length::len(file)?;
	if range.end > file_len {
		return Err(std::io::Error::new(
			ErrorKind::InvalidInput,
			"range extends beyond file length",
		));
	}

	let old_len = range.end - range.start;
	let replacement_len = u64::try_from(replacement.len())
		.map_err(|_| std::io::Error::new(ErrorKind::InvalidInput, "replacement is too large"))?;

	let mut buffer = vec![0_u8; MOVE_BUFFER_SIZE];

	match replacement_len.cmp(&old_len) {
		std::cmp::Ordering::Greater => {
			let difference = replacement_len - old_len;
			// The ranges overlap, so move the tail backwards from EOF before writing metadata.
			extend_storage(file, difference, &buffer)?;
			shift_right(file, range.end, file_len, difference, &mut buffer)?;
		},
		std::cmp::Ordering::Less => {
			let difference = old_len - replacement_len;
			// Move forwards from the metadata boundary so writes cannot clobber unread tail data.
			shift_left(file, range.end, file_len, difference, &mut buffer)?;
			file.truncate(file_len - difference)?;
		},
		std::cmp::Ordering::Equal => {},
	}

	file.seek(SeekFrom::Start(range.start))?;
	file.write_all(replacement)?;

	Ok(())
}

fn extend_storage<F>(file: &mut F, amount: u64, zeros: &[u8]) -> std::io::Result<()>
where
	F: FileLike,
{
	file.seek(SeekFrom::End(0))?;

	let mut remaining = amount;
	while remaining != 0 {
		let chunk_len = usize::try_from(remaining.min(zeros.len() as u64))
			.expect("chunk length is bounded by the in-memory buffer");
		file.write_all(&zeros[..chunk_len])?;
		remaining -= chunk_len as u64;
	}

	Ok(())
}

fn shift_right<F>(
	file: &mut F,
	start: u64,
	end: u64,
	amount: u64,
	buffer: &mut [u8],
) -> std::io::Result<()>
where
	F: FileLike,
{
	let mut cursor = end;

	while cursor > start {
		let chunk_len = usize::try_from((cursor - start).min(buffer.len() as u64))
			.expect("chunk length is bounded by the in-memory buffer");
		let source = cursor - chunk_len as u64;

		file.seek(SeekFrom::Start(source))?;
		file.read_exact(&mut buffer[..chunk_len])?;

		file.seek(SeekFrom::Start(source + amount))?;
		file.write_all(&buffer[..chunk_len])?;

		cursor = source;
	}

	Ok(())
}

fn shift_left<F>(
	file: &mut F,
	start: u64,
	end: u64,
	amount: u64,
	buffer: &mut [u8],
) -> std::io::Result<()>
where
	F: FileLike,
{
	let mut cursor = start;

	while cursor < end {
		let chunk_len = usize::try_from((end - cursor).min(buffer.len() as u64))
			.expect("chunk length is bounded by the in-memory buffer");

		file.seek(SeekFrom::Start(cursor))?;
		file.read_exact(&mut buffer[..chunk_len])?;

		file.seek(SeekFrom::Start(cursor - amount))?;
		file.write_all(&buffer[..chunk_len])?;

		cursor += chunk_len as u64;
	}

	Ok(())
}

fn encode_tag<'a, II, IP>(
	vendor: &str,
	mut comments_peek: Peekable<&mut II>,
	pictures_peek: Peekable<&mut IP>,
) -> Result<Vec<Block>, FileEncodingError>
where
	II: Iterator<Item = (&'a str, &'a str)>,
	IP: Iterator<Item = (&'a Picture, PictureInformation)>,
{
	let mut metadata_blocks = Vec::new();

	if comments_peek.peek().is_some() {
		metadata_blocks.push(Block::new_comments(vendor, &mut comments_peek)?);
	}

	for (picture, info) in pictures_peek {
		metadata_blocks.push(Block::new_picture(picture, info)?);
	}

	Ok(metadata_blocks)
}

#[cfg(test)]
mod tests {
	use super::*;

	use std::io::Cursor;

	fn apply_range(input: Vec<u8>, range: Range<usize>, replacement: &[u8]) {
		let mut expected = input.clone();
		drop(expected.splice(range.clone(), replacement.iter().copied()));

		let mut cursor = Cursor::new(input);
		replace_range(
			&mut cursor,
			(range.start as u64)..(range.end as u64),
			replacement,
		)
		.expect("range replacement should succeed");

		let actual = cursor.into_inner();
		assert_eq!(actual, expected);
	}

	#[test]
	fn replace_range_equal_size() {
		apply_range(b"0123456789".to_vec(), 2..5, b"XYZ");
	}

	#[test]
	fn replace_range_grows() {
		apply_range(b"0123456789".to_vec(), 2..5, b"abcdef");
	}

	#[test]
	fn replace_range_shrinks() {
		apply_range(b"0123456789".to_vec(), 2..8, b"X");
	}

	#[test]
	fn replace_range_grows_across_multiple_buffers() {
		let mut input = b"prefix".to_vec();
		input.extend((0..(MOVE_BUFFER_SIZE * 3 + 17)).map(|index| (index % 251) as u8));

		apply_range(input, 1..4, b"a much longer metadata replacement");
	}

	#[test]
	fn replace_range_shrinks_across_multiple_buffers() {
		let mut input = b"prefix".to_vec();
		input.extend((0..(MOVE_BUFFER_SIZE * 3 + 17)).map(|index| (index % 251) as u8));

		apply_range(input, 1..4, b"x");
	}
}
