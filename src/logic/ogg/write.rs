use super::{page_from_packet, verify_signature};
use crate::error::{LoftyError, Result};
use crate::logic::ogg::constants::OPUSTAGS;
use crate::logic::ogg::constants::VORBIS_COMMENT_HEAD;
use crate::types::item::{ItemValue, TagItem};
use crate::types::tag::{Tag, TagType};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::convert::TryFrom;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};

use ogg_pager::Page;

pub(crate) fn create_comments(packet: &mut Vec<u8>, items: &[TagItem]) {
	for item in items {
		if let ItemValue::Text(value) = item.value() {
			let comment = format!(
				"{}={}",
				item.key().map_key(&TagType::VorbisComments).unwrap(),
				value
			);
			let comment_b = comment.as_bytes();

			let bytes_len = comment_b.len();

			if u32::try_from(bytes_len as u64).is_ok() {
				packet.extend((bytes_len as u32).to_le_bytes().iter());
				packet.extend(comment_b.iter());
			}
		}
	}
}

fn create_pages(tag: &Tag, writer: &mut Vec<u8>) -> Result<Vec<Page>> {
	let item_count = tag.item_count() + tag.picture_count();

	writer.write_u32::<LittleEndian>(item_count)?;
	create_comments(writer, tag.items());

	for pic in tag.pictures() {
		let picture = format!(
			"METADATA_BLOCK_PICTURE={}",
			base64::encode(pic.as_flac_bytes())
		);
		let picture_b = picture.as_bytes();
		let bytes_len = picture_b.len();

		if u32::try_from(bytes_len as u64).is_ok() {
			writer.write_u32::<LittleEndian>(bytes_len as u32)?;
			writer.write_all(picture_b)?;
		}
	}

	page_from_packet(writer)
}

pub(in crate::logic) fn write_to(data: &mut File, tag: &Tag, sig: &[u8]) -> Result<()> {
	if tag.tag_type() != &TagType::VorbisComments {
		return Err(LoftyError::UnsupportedTag);
	}

	let first_page = Page::read(data, false)?;

	let ser = first_page.serial;

	let mut writer = Vec::new();
	writer.write_all(&*first_page.as_bytes())?;

	let first_md_page = Page::read(data, false)?;
	verify_signature(&first_md_page, sig)?;

	// Retain the file's vendor string
	let md_reader = &mut &first_md_page.content[sig.len()..];

	let vendor_len = md_reader.read_u32::<LittleEndian>()?;
	let mut vendor = vec![0; vendor_len as usize];
	md_reader.read_exact(&mut vendor)?;

	let mut packet = Vec::new();
	packet.write_all(sig)?;
	packet.write_u32::<LittleEndian>(vendor_len)?;
	packet.write_all(&vendor)?;

	let mut pages = create_pages(tag, &mut packet)?;

	match sig {
		VORBIS_COMMENT_HEAD => {
			super::vorbis::write::write_to(
				data,
				&mut writer,
				first_md_page.content,
				ser,
				&mut pages,
			)?;
		}
		OPUSTAGS => {
			super::opus::write::write_to(data, &mut writer, ser, &mut pages)?;
		}
		_ => unreachable!(),
	}

	data.seek(SeekFrom::Start(0))?;
	data.set_len(first_page.end as u64)?;
	data.write_all(&*writer)?;

	Ok(())
}
