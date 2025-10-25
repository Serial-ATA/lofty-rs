use super::block::{BLOCK_ID_PADDING, BLOCK_ID_PICTURE, BLOCK_ID_VORBIS_COMMENTS, Block};
use super::read::verify_flac;
use crate::config::WriteOptions;
use crate::error::{LoftyError, Result};
use crate::macros::{err, try_vec};
use crate::ogg::tag::VorbisCommentsRef;
use crate::picture::{Picture, PictureInformation};
use crate::tag::{Tag, TagType};
use crate::util::io::{FileLike, Length, Truncate};

use std::borrow::Cow;
use std::io::{Cursor, Read, Seek, SeekFrom};
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

/// The location of the last metadata block
///
/// Depending on the order of the metadata blocks, the metadata present for writing, and the [`WriteOptions`], the
/// writer will have to determine when to set the `Last-metadata-block` flag. This is because the writer
/// doesn't fully parse the file, it only writes what has changed.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum LastBlock {
	/// The last block in the current file is the stream info
	StreamInfo,
	/// The last block will be the newly written Vorbis Comments
	Comments,
	/// The last block will be the final newly written picture
	Picture,
	/// The last block will be the newly written padding block, if the user allows for padding (most common case)
	Padding,
	/// The last block already exists in the stream and will not be touched while writing, so nothing
	/// to do
	Other,
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

	let stream_info = verify_flac(&mut cursor)?;
	let stream_info_start = stream_info.start as usize;
	let stream_info_end = stream_info.end as usize;

	let BlockCollector {
		vendor,
		mut blocks,
		mut last_block_location,
		mut last_block_replaced,
	} = BlockCollector::collect(&mut cursor, stream_info)?;

	if let Some(vendor) = vendor {
		tag.vendor = vendor;
	}

	let mut comments_peek = (&mut tag.items).peekable();
	let mut pictures_peek = (&mut tag.pictures).peekable();

	let has_comments = comments_peek.peek().is_some();
	let has_pictures = pictures_peek.peek().is_some();

	// Attempting to strip an already empty file
	let has_blocks_to_remove = blocks.iter().any(|b| b.for_removal);
	if !has_blocks_to_remove && !has_comments && !has_pictures {
		log::debug!("Nothing to do");
		return Ok(());
	}

	// TODO: We need to actually use padding (https://github.com/Serial-ATA/lofty-rs/issues/445)
	let will_write_padding =
		last_block_location != LastBlock::Padding && write_options.preferred_padding.is_some();
	let mut file_bytes = cursor.into_inner();

	if last_block_location == LastBlock::StreamInfo {
		if has_comments || has_pictures {
			// Stream info is *always* the first block, so it'll always be replaced if we add more blocks
			last_block_replaced = true;

			if has_pictures {
				last_block_location = LastBlock::Picture;
			} else {
				last_block_location = LastBlock::Comments;
			}
		} else if !will_write_padding {
			// Otherwise, we set the `Last-metadata-block` flag
			file_bytes[stream_info_start] |= 0x80;
		}
	}

	let last_metadata_block = &blocks
		.last()
		.expect("should always have at least one block")
		.block;
	if will_write_padding {
		if let Some(preferred_padding) = write_options.preferred_padding {
			log::warn!("File is missing a PADDING block. Adding one");

			// `PADDING` always goes last
			last_block_replaced = true;

			let mut padding_block = Block::new_padding(preferred_padding as usize)?;
			padding_block.last = true;

			let mut encoded_padding_block = Vec::with_capacity(padding_block.len() as usize);
			padding_block.write_to(&mut encoded_padding_block)?;

			let metadata_end = last_metadata_block.end as usize;
			file_bytes.splice(metadata_end..metadata_end, encoded_padding_block);
		}
	}

	// For example, if the file's previous last block was `STREAMINFO`, and we wrote a `VORBIS_COMMENT` block,
	// then we unset the `Last-metadata-block` flag on the `STREAMINFO`.
	if last_block_replaced {
		file_bytes[last_metadata_block.start as usize] = last_metadata_block.ty;
	}

	let metadata_blocks = encode_tag(
		&tag.vendor,
		comments_peek,
		pictures_peek,
		last_block_location,
	)?;

	blocks.reverse();
	for block in blocks {
		if !block.for_removal {
			continue;
		}

		file_bytes.drain(block.block.start as usize..block.block.end as usize);
	}

	file_bytes.splice(stream_info_end..stream_info_end, metadata_blocks);

	file.seek(SeekFrom::Start(0))?;
	file.write_all(&file_bytes)?;

	Ok(())
}

fn encode_tag<'a, II, IP>(
	vendor: &str,
	mut comments_peek: Peekable<&mut II>,
	mut pictures_peek: Peekable<&mut IP>,
	last_block_location: LastBlock,
) -> Result<Vec<u8>>
where
	II: Iterator<Item = (&'a str, &'a str)>,
	IP: Iterator<Item = (&'a Picture, PictureInformation)>,
{
	let mut metadata_blocks = Cursor::new(Vec::new());

	if comments_peek.peek().is_some() {
		let mut block = Block::new_comments(vendor, &mut comments_peek)?;
		if last_block_location == LastBlock::Comments {
			block.last = true;
		}

		let block_size = block.write_to(&mut metadata_blocks)?;

		log::trace!("Wrote a comment block, size: {block_size}",);
	}

	loop {
		let Some((picture, info)) = pictures_peek.next() else {
			break;
		};

		let is_last_picture = pictures_peek.peek().is_none();

		let mut block = Block::new_picture(picture, info)?;
		if is_last_picture && last_block_location == LastBlock::Picture {
			block.last = true;
		}

		let block_size = block.write_to(&mut metadata_blocks)?;
		log::trace!("Wrote a picture block, size: {block_size}");
	}

	Ok(metadata_blocks.into_inner())
}

struct CollectedBlock {
	for_removal: bool,
	block: Block,
}

struct BlockCollector {
	vendor: Option<Cow<'static, str>>,
	blocks: Vec<CollectedBlock>,
	last_block_location: LastBlock,
	/// The *current* last block in the file will be replaced by whatever we end up writing, so we
	/// need to set the `Last-metadata-block` flag on it
	last_block_replaced: bool,
}

impl BlockCollector {
	/// Collect all the blocks in the file and mark which ones will be removed by this write
	fn collect<R>(reader: &mut R, stream_info: Block) -> Result<Self>
	where
		R: Read + Seek,
	{
		// Vendor string from the file, if it exists
		let mut vendor = None;

		let mut last_block_location = LastBlock::StreamInfo;
		let mut last_block_replaced = false;

		let mut is_last_block = stream_info.last;
		let mut blocks = Vec::new();
		blocks.push(CollectedBlock {
			for_removal: false,
			block: stream_info,
		});

		while !is_last_block {
			let block = Block::read(reader, |block_ty| block_ty == BLOCK_ID_VORBIS_COMMENTS)?;
			is_last_block = block.last;

			let mut for_removal = false;
			match block.ty {
				BLOCK_ID_VORBIS_COMMENTS => {
					for_removal = true;

					// Retain the original vendor string
					let reader = &mut &block.content[..];

					let vendor_len = reader.read_u32::<LittleEndian>()?;
					let mut vendor_raw = try_vec![0; vendor_len as usize];
					reader.read_exact(&mut vendor_raw)?;

					match String::from_utf8(vendor_raw) {
						Ok(vendor_str) => vendor = Some(Cow::Owned(vendor_str)),
						// TODO: Error on strict?
						Err(_) => {
							log::warn!("FLAC vendor string is not valid UTF-8, not re-using");
							vendor = Some(Cow::Borrowed(""));
						},
					}

					if is_last_block {
						last_block_replaced = true;
					}
				},
				BLOCK_ID_PICTURE => {
					for_removal = true;
					if is_last_block {
						last_block_replaced = true;
					}
				},
				BLOCK_ID_PADDING => {
					if is_last_block {
						last_block_location = LastBlock::Padding;
					} else {
						for_removal = true;
					}
				},
				_ => {
					if is_last_block {
						last_block_location = LastBlock::Other;
					}
				},
			}

			blocks.push(CollectedBlock { for_removal, block });
		}

		Ok(Self {
			vendor,
			blocks,
			last_block_location,
			last_block_replaced,
		})
	}
}
