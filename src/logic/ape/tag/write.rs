use super::read::read_ape_tag;
use crate::error::{LoftyError, Result};
use crate::logic::ape::constants::APE_PREAMBLE;
use crate::logic::ape::tag::item::ApeItemRef;
use crate::logic::ape::tag::ApeTagRef;
use crate::logic::id3::v2::find_id3v2;
use crate::logic::id3::{find_id3v1, find_lyrics3v2};
use crate::probe::Probe;
use crate::types::file::FileType;
use crate::types::item::ItemValueRef;

use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use byteorder::{LittleEndian, WriteBytesExt};

pub(in crate::logic) fn write_to(data: &mut File, tag: &ApeTagRef) -> Result<()> {
	match Probe::new().file_type(data) {
		Some(ft) if ft == FileType::APE || ft == FileType::MP3 => {},
		_ => return Err(LoftyError::UnsupportedTag),
	}

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
		existing.items.retain(|_i, v| v.read_only);

		if !existing.items.is_empty() {
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

		existing.items.retain(|_, v| v.read_only);

		if !existing.items.is_empty() {
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
	let tag = if let Some(read_only) = read_only {
		create_ape_tag(&Into::<ApeTagRef>::into(&read_only).items)?
	} else {
		create_ape_tag(&tag.items)?
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

fn create_ape_tag(items: &HashMap<&str, ApeItemRef>) -> Result<Vec<u8>> {
	// Unnecessary to write anything if there's no metadata
	if items.is_empty() {
		Ok(Vec::<u8>::new())
	} else {
		let mut tag = Cursor::new(Vec::<u8>::new());

		let item_count = items.len() as u32;

		for (k, v) in items {
			let (mut flags, value) = match v.value {
				ItemValueRef::Binary(value) => {
					tag.write_u32::<LittleEndian>(value.len() as u32)?;

					(1_u32 << 1, value)
				},
				ItemValueRef::Text(value) => {
					tag.write_u32::<LittleEndian>(value.len() as u32)?;

					(0_u32, value.as_bytes())
				},
				ItemValueRef::Locator(value) => {
					tag.write_u32::<LittleEndian>(value.len() as u32)?;

					(2_u32 << 1, value.as_bytes())
				},
			};

			if v.read_only {
				flags |= 1_u32
			}

			tag.write_u32::<LittleEndian>(flags)?;
			tag.write_all(k.as_bytes())?;
			tag.write_u8(0)?;
			tag.write_all(value)?;
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
