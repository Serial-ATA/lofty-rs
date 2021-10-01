use super::verify_signature;
use crate::error::{LoftyError, Result};
use crate::picture::Picture;
use crate::types::item::{ItemKey, ItemValue, TagItem};
use crate::types::tag::{Tag, TagType};

use std::io::{Read, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};
use ogg_pager::Page;

pub type OGGTags = (String, Tag, Page);

pub(crate) fn read_comments<R>(data: &mut R, tag: &mut Tag) -> Result<String>
where
	R: Read,
{
	let vendor_len = data.read_u32::<LittleEndian>()?;

	let mut vendor = vec![0; vendor_len as usize];
	data.read_exact(&mut vendor)?;

	let vendor = match String::from_utf8(vendor) {
		Ok(v) => v,
		Err(_) => return Err(LoftyError::Ogg("File has an invalid vendor string")),
	};

	let comments_total_len = data.read_u32::<LittleEndian>()?;

	for _ in 0..comments_total_len {
		let comment_len = data.read_u32::<LittleEndian>()?;

		let mut comment_bytes = vec![0; comment_len as usize];
		data.read_exact(&mut comment_bytes)?;

		let comment = String::from_utf8(comment_bytes)?;

		let split: Vec<&str> = comment.splitn(2, '=').collect();

		if split[0] == "METADATA_BLOCK_PICTURE" {
			tag.push_picture(Picture::from_flac_bytes(split[1].as_bytes())?)
		} else {
			// It's safe to unwrap here since any unknown key is wrapped in ItemKey::Unknown
			tag.insert_item(TagItem::new(
				ItemKey::from_key(&TagType::VorbisComments, split[0]).unwrap(),
				ItemValue::Text(split[1].to_string()),
			));
		}
	}

	Ok(vendor)
}

pub(crate) fn read_from<T>(data: &mut T, header_sig: &[u8], comment_sig: &[u8]) -> Result<OGGTags>
where
	T: Read + Seek,
{
	let first_page = Page::read(data, false)?;
	verify_signature(&first_page, header_sig)?;

	let md_page = Page::read(data, false)?;
	verify_signature(&md_page, comment_sig)?;

	let mut md_pages: Vec<u8> = Vec::new();

	md_pages.extend(md_page.content[comment_sig.len()..].iter());

	while let Ok(page) = Page::read(data, false) {
		if md_pages.len() > 125_829_120 {
			return Err(LoftyError::TooMuchData);
		}

		if page.header_type == 1 {
			md_pages.extend(page.content.iter());
		} else {
			data.seek(SeekFrom::Start(page.start))?;
			break;
		}
	}

	let mut tag = Tag::new(TagType::VorbisComments);

	let reader = &mut &md_pages[..];
	let vendor = read_comments(reader, &mut tag)?;

	Ok((vendor, tag, first_page))
}
