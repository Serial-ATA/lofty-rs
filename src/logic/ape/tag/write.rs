use super::read::read_ape_tag;
use crate::error::{LoftyError, Result};
use crate::logic::ape::constants::APE_PREAMBLE;
use crate::logic::id3::find_lyrics3v2;
use crate::logic::id3::v1::find_id3v1;
use crate::logic::id3::v2::find_id3v2;
use crate::types::picture::Picture;
use crate::types::tag::{ItemValue, Tag, TagItem, TagType};

use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use byteorder::{LittleEndian, WriteBytesExt};

pub(crate) fn write_to(data: &mut File, tag: &Tag) -> Result<()> {
	// We don't actually need the ID3v2 tag, but reading it will seek to the end of it if it exists
	find_id3v2(data, false)?;

	let mut ape_preamble = [0; 8];
	data.read_exact(&mut ape_preamble)?;

	// We have to check the APE tag for any read only items first
	let mut read_only = None;

	// An APE tag in the beginning of a file is against the spec
	// If one is found, it'll be removed and rewritten at the bottom, where it should be
	let mut header_ape_tag = (false, (0, 0));

	if &ape_preamble == APE_PREAMBLE {
		let start = data.seek(SeekFrom::Current(-8))?;

		data.seek(SeekFrom::Current(8))?;
		let (mut existing, size) = read_ape_tag(data, false)?;

		// Only keep metadata around that's marked read only
		existing.retain(|i| i.flags().read_only);

		if existing.item_count() > 0 {
			read_only = Some(existing)
		}

		header_ape_tag = (true, (start, start + u64::from(size)))
	} else {
		data.seek(SeekFrom::Current(-8))?;
	}

	// Skip over ID3v1 and Lyrics3v2 tags
	find_id3v1(data, false)?;
	find_lyrics3v2(data)?;

	// In case there's no ape tag already, this is the spot it belongs
	let ape_position = data.seek(SeekFrom::Current(0))?;

	// Now search for an APE tag at the end
	data.seek(SeekFrom::Current(-32))?;

	data.read_exact(&mut ape_preamble)?;

	let mut ape_tag_location = None;

	// Also check this tag for any read only items
	if &ape_preamble == APE_PREAMBLE {
		let start = data.seek(SeekFrom::Current(0))? as usize + 24;

		let (mut existing, size) = read_ape_tag(data, true)?;

		existing.retain(|i| i.flags().read_only);

		if existing.item_count() > 0 {
			read_only = Some(existing)
		}

		// Since the "start" was really at the end of the tag, this sanity check seems necessary
		if let Some(start) = start.checked_sub(size as usize) {
			ape_tag_location = Some(start..start + size as usize);
		} else {
			return Err(LoftyError::Ape("File has a tag with an invalid size"));
		}
	}

	// Preserve any metadata marked as read only
	// If there is any read only metadata, we will have to clone the TagItems
	let tag = if let Some(read_only) = read_only {
		use std::collections::HashSet;

		let mut items = [read_only.items(), tag.items()].concat();

		let mut unique_items = HashSet::new();
		items.retain(|i| unique_items.insert(i.clone()));

		create_ape_tag(&items, tag.pictures())?
	} else {
		create_ape_tag(tag.items(), tag.pictures())?
	};

	data.seek(SeekFrom::Start(0))?;

	let mut file_bytes = Vec::new();
	data.read_to_end(&mut file_bytes)?;

	// Write the tag in the appropriate place
	if let Some(range) = ape_tag_location {
		file_bytes.splice(range, tag);
	} else {
		file_bytes.splice(ape_position as usize..ape_position as usize, tag);
	}

	// Now, if there was a tag at the beginning, remove it
	if header_ape_tag.0 {
		file_bytes.drain(header_ape_tag.1 .0 as usize..header_ape_tag.1 .1 as usize);
	}

	data.seek(SeekFrom::Start(0))?;
	data.set_len(0)?;
	data.write_all(&*file_bytes)?;

	Ok(())
}

fn create_ape_tag(items: &[TagItem], pictures: &[Picture]) -> Result<Vec<u8>> {
	// Unnecessary to write anything if there's no metadata
	if items.is_empty() && pictures.is_empty() {
		Ok(Vec::<u8>::new())
	} else {
		let mut tag = Cursor::new(Vec::<u8>::new());

		let item_count = (items.len() + pictures.len()) as u32;

		for item in items {
			let (size, flags, value) = match item.value() {
				ItemValue::Binary(value) => {
					let mut flags = 1_u32 << 1;

					if item.flags().read_only {
						flags |= 1_u32
					}

					(value.len() as u32, flags, value.as_slice())
				},
				ItemValue::Text(value) => {
					let value = value.as_bytes();

					let mut flags = 0_u32;

					if item.flags().read_only {
						flags |= 1_u32
					}

					(value.len() as u32, flags, value)
				},
				ItemValue::Locator(value) => {
					let mut flags = 2_u32 << 1;

					if item.flags().read_only {
						flags |= 1_u32
					}

					(value.len() as u32, flags, value.as_bytes())
				},
				_ => continue,
			};

			tag.write_u32::<LittleEndian>(size)?;
			tag.write_u32::<LittleEndian>(flags)?;
			tag.write_all(item.key().map_key(&TagType::Ape).unwrap().as_bytes())?;
			tag.write_u8(0)?;
			tag.write_all(value)?;
		}

		for pic in pictures {
			let key = pic.pic_type.as_ape_key();
			let bytes = pic.as_ape_bytes();
			// Binary item
			let flags = 1_u32 << 1;

			tag.write_u32::<LittleEndian>(bytes.len() as u32)?;
			tag.write_u32::<LittleEndian>(flags)?;
			tag.write_all(key.as_bytes())?;
			tag.write_u8(0)?;
			tag.write_all(&bytes)?;
		}

		let size = tag.get_ref().len();

		if size as u64 + 32 > u64::from(u32::MAX) {
			return Err(LoftyError::TooMuchData);
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
		footer.write_u32::<LittleEndian>((1_u32 << 30) | (1_u32 << 31))?;
		// The header/footer must end in 8 bytes of zeros
		footer.write_u64::<LittleEndian>(0)?;

		tag.write_all(footer.get_ref())?;

		let mut tag = tag.into_inner();

		// The header is exactly the same as the footer, except for the flags
		// Just reuse the footer and overwrite the flags
		footer.seek(SeekFrom::Current(-12))?;
		// Bit 29 set: this is the header
		// Bit 30 set: tag contains a footer
		// Bit 31 set: tag contains a header
		footer.write_u32::<LittleEndian>((1_u32 << 29) | (1_u32 << 30) | (1_u32 << 31))?;

		let header = footer.into_inner();

		tag.splice(0..0, header.to_vec());

		Ok(tag)
	}
}
