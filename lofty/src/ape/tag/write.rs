use super::ApeTagRef;
use super::item::ApeItemRef;
use crate::ape::constants::APE_PREAMBLE;
use crate::ape::tag::read;
use crate::config::{ParseOptions, WriteOptions};
use crate::error::{LoftyError, Result};
use crate::id3::{FindId3v2Config, find_id3v1, find_id3v2, find_lyrics3v2};
use crate::macros::{decode_err, err};
use crate::probe::Probe;
use crate::tag::item::ItemValueRef;
use crate::util::io::{FileLike, Truncate};

use std::io::{Cursor, Seek, SeekFrom, Write};

use byteorder::{LittleEndian, WriteBytesExt};

#[allow(clippy::shadow_unrelated)]
pub(crate) fn write_to<'a, F, I>(
	file: &mut F,
	tag_ref: &mut ApeTagRef<'a, I>,
	write_options: WriteOptions,
) -> Result<()>
where
	I: Iterator<Item = ApeItemRef<'a>>,
	F: FileLike,
	LoftyError: From<<F as Truncate>::Error>,
{
	let probe = Probe::new(file).guess_file_type()?;

	match probe.file_type() {
		Some(ft) if super::ApeTag::SUPPORTED_FORMATS.contains(&ft) => {},
		_ => err!(UnsupportedTag),
	}

	let file = probe.into_inner();

	// We don't actually need the ID3v2 tag, but reading it will seek to the end of it if it exists
	find_id3v2(file, FindId3v2Config::NO_READ_TAG)?;

	let mut ape_preamble = [0; 8];
	file.read_exact(&mut ape_preamble)?;

	// We have to check the APE tag for any read only items first
	let mut read_only = None;

	// An APE tag in the beginning of a file is against the spec
	// If one is found, it'll be removed and rewritten at the bottom, where it should be
	let mut header_ape_tag = (false, (0, 0));

	// TODO: Forcing the use of ParseOptions::default()
	let parse_options = ParseOptions::new();

	let start = file.stream_position()?;
	match read::read_ape_tag(file, false, parse_options)? {
		(Some(mut existing_tag), Some(header)) => {
			if write_options.respect_read_only {
				// Only keep metadata around that's marked read only
				existing_tag.items.retain(|i| i.read_only);

				if !existing_tag.items.is_empty() {
					read_only = Some(existing_tag)
				}
			}

			header_ape_tag = (true, (start, start + u64::from(header.size)))
		},
		_ => {
			file.seek(SeekFrom::Current(-8))?;
		},
	}

	// Skip over ID3v1 and Lyrics3v2 tags
	find_id3v1(file, false, parse_options.parsing_mode)?;
	find_lyrics3v2(file)?;

	// In case there's no ape tag already, this is the spot it belongs
	let ape_position = file.stream_position()?;

	// Now search for an APE tag at the end
	file.seek(SeekFrom::Current(-32))?;

	let mut ape_tag_location = None;

	// Also check this tag for any read only items
	let start = file.stream_position()? as usize + 32;
	// TODO: Forcing the use of ParseOptions::default()
	if let (Some(mut existing_tag), Some(header)) =
		read::read_ape_tag(file, true, ParseOptions::new())?
	{
		if write_options.respect_read_only {
			existing_tag.items.retain(|i| i.read_only);

			if !existing_tag.items.is_empty() {
				read_only = match read_only {
					Some(mut read_only) => {
						read_only.items.extend(existing_tag.items);
						Some(read_only)
					},
					None => Some(existing_tag),
				}
			}
		}

		// Since the "start" was really at the end of the tag, this sanity check seems necessary
		let size = header.size;
		if let Some(start) = start.checked_sub(size as usize) {
			ape_tag_location = Some(start..start + size as usize);
		} else {
			decode_err!(@BAIL Ape, "File has a tag with an invalid size");
		}
	}

	// Preserve any metadata marked as read only
	let tag;
	if let Some(read_only) = read_only {
		tag = create_ape_tag(
			tag_ref,
			read_only.items.iter().map(Into::into),
			write_options,
		)?;
	} else {
		tag = create_ape_tag(tag_ref, std::iter::empty(), write_options)?;
	}

	file.rewind()?;

	let mut file_bytes = Vec::new();
	file.read_to_end(&mut file_bytes)?;

	// Write the tag in the appropriate place
	if let Some(range) = ape_tag_location {
		file_bytes.splice(range, tag);
	} else {
		file_bytes.splice(ape_position as usize..ape_position as usize, tag);
	}

	// Now, if there was a tag at the beginning, remove it
	if header_ape_tag.0 {
		file_bytes.drain(header_ape_tag.1.0 as usize..header_ape_tag.1.1 as usize);
	}

	file.rewind()?;
	file.truncate(0)?;
	file.write_all(&file_bytes)?;

	Ok(())
}

pub(super) fn create_ape_tag<'a, 'b, I, R>(
	tag: &mut ApeTagRef<'a, I>,
	mut read_only: R,
	write_options: WriteOptions,
) -> Result<Vec<u8>>
where
	I: Iterator<Item = ApeItemRef<'a>>,
	R: Iterator<Item = ApeItemRef<'b>>,
{
	let items = &mut tag.items;
	let mut peek = items.peekable();

	// Unnecessary to write anything if there's no metadata
	if peek.peek().is_none() {
		return Ok(Vec::<u8>::new());
	}

	if read_only.next().is_some() && write_options.respect_read_only {
		// TODO: Implement retaining read only items
		log::warn!("Retaining read only items is not supported yet");
		drop(read_only);
	}

	let mut tag_write = Cursor::new(Vec::<u8>::new());

	let mut item_count = 0_u32;

	for item in peek {
		let (mut flags, value) = match item.value {
			ItemValueRef::Binary(value) => {
				tag_write.write_u32::<LittleEndian>(value.len() as u32)?;

				(1_u32 << 1, value)
			},
			ItemValueRef::Text(ref value) => {
				tag_write.write_u32::<LittleEndian>(value.len() as u32)?;

				(0_u32, value.as_bytes())
			},
			ItemValueRef::Locator(value) => {
				tag_write.write_u32::<LittleEndian>(value.len() as u32)?;

				(2_u32 << 1, value.as_bytes())
			},
		};

		if item.read_only {
			flags |= 1_u32
		}

		tag_write.write_u32::<LittleEndian>(flags)?;
		tag_write.write_all(item.key.as_bytes())?;
		tag_write.write_u8(0)?;
		tag_write.write_all(value)?;

		item_count += 1;
	}

	let size = tag_write.get_ref().len();

	if size as u64 + 32 > u64::from(u32::MAX) {
		err!(TooMuchData);
	}

	let mut footer = [0_u8; 32];
	let mut footer = Cursor::new(&mut footer[..]);

	footer.write_all(APE_PREAMBLE)?;
	// This is the APE tag version
	// Even if we read a v1 tag, we end up adding a header anyway
	footer.write_u32::<LittleEndian>(2000)?;
	// The total size includes the 32 bytes of the footer
	footer.write_u32::<LittleEndian>((size + 32) as u32)?;
	footer.write_u32::<LittleEndian>(item_count)?;
	// Bit 29 unset: this is the footer
	// Bit 30 set: tag contains a footer
	// Bit 31 set: tag contains a header
	let mut footer_flags = (1_u32 << 30) | (1_u32 << 31);

	if tag.read_only {
		// Bit 0 set: tag is read only
		footer_flags |= 1
	}

	footer.write_u32::<LittleEndian>(footer_flags)?;
	// The header/footer must end in 8 bytes of zeros
	footer.write_u64::<LittleEndian>(0)?;

	tag_write.write_all(footer.get_ref())?;

	let mut tag_write = tag_write.into_inner();

	// The header is exactly the same as the footer, except for the flags
	// Just reuse the footer and overwrite the flags
	footer.seek(SeekFrom::Current(-12))?;
	// Bit 29 set: this is the header
	// Bit 30 set: tag contains a footer
	// Bit 31 set: tag contains a header
	let mut header_flags = (1_u32 << 29) | (1_u32 << 30) | (1_u32 << 31);

	if tag.read_only {
		// Bit 0 set: tag is read only
		header_flags |= 1
	}

	footer.write_u32::<LittleEndian>(header_flags)?;

	let header = footer.into_inner();

	tag_write.splice(0..0, header.to_vec());

	Ok(tag_write)
}
