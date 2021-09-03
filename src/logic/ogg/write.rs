use super::{opus, page_from_packet, verify_signature, vorbis};
use crate::error::Result;
use crate::logic::ogg::constants::OPUSTAGS;
use crate::logic::ogg::constants::VORBIS_COMMENT_HEAD;
use crate::types::item::ItemKey;
use crate::types::tag::{ItemValue, Tag, TagItem, TagType};

use std::convert::TryFrom;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};

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

pub(crate) fn create_pages(file: &mut File, sig: &[u8], tag: &Tag) -> Result<()> {
	let mut packet = Vec::new();

	packet.extend(sig.iter());

	if let Some(ItemValue::Text(vendor)) = tag
		.get_item_ref(&ItemKey::EncoderSoftware)
		.map(TagItem::value)
	{
		packet.extend((vendor.len() as u32).to_le_bytes().iter());
		packet.extend(vendor.as_bytes().iter());
	} else {
		packet.extend([0, 0, 0, 0].iter())
	};

	let item_count = tag.item_count() + tag.picture_count();

	packet.extend(item_count.to_le_bytes().iter());
	create_comments(&mut packet, tag.items());

	for pic in tag.pictures() {
		let picture = format!(
			"METADATA_BLOCK_PICTURE={}",
			base64::encode(pic.as_flac_bytes())
		);
		let picture_b = picture.as_bytes();
		let bytes_len = picture_b.len();

		if u32::try_from(bytes_len as u64).is_ok() {
			packet.extend((bytes_len as u32).to_le_bytes().iter());
			packet.extend(picture_b.iter());
		}
	}

	let mut pages = page_from_packet(&mut *packet)?;
	write_to(file, &mut pages, sig)?;

	Ok(())
}

fn write_to(mut data: &mut File, pages: &mut [Page], sig: &[u8]) -> Result<()> {
	let first_page = Page::read(&mut data, false)?;

	let ser = first_page.serial;

	let mut writer = Vec::new();
	writer.write_all(&*first_page.as_bytes())?;

	let first_md_page = Page::read(&mut data, false)?;
	verify_signature(&first_md_page, sig)?;

	match sig {
		VORBIS_COMMENT_HEAD => {
			vorbis::write_to(data, &mut writer, first_md_page.content, ser, pages)?;
		},
		OPUSTAGS => {
			opus::write_to(data, &mut writer, ser, pages)?;
		},
		_ => unreachable!(),
	}

	data.seek(SeekFrom::Start(0))?;
	data.set_len(first_page.end as u64)?;
	data.write_all(&*writer)?;

	Ok(())
}
