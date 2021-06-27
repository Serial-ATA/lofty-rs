use crate::components::logic::constants::OPUSHEAD;
use crate::components::logic::ogg::{is_metadata, reach_metadata};
use crate::{LoftyError, OggFormat, Result, Picture};

use std::collections::HashMap;
use std::io::{Read, Seek};

use byteorder::{LittleEndian, ReadBytesExt};
use ogg_pager::Page;

pub type OGGTags = (String, Vec<Picture>, HashMap<String, String>, OggFormat);

pub(crate) fn read_from<T>(mut data: T, header_sig: &[u8], comment_sig: &[u8]) -> Result<OGGTags>
where
	T: Read + Seek,
{
	reach_metadata(&mut data, header_sig)?;

	let md_page = Page::read(&mut data)?;
	is_metadata(&md_page, comment_sig)?;

	let mut md_pages: Vec<u8> = Vec::new();

	md_pages.extend(md_page.content[comment_sig.len()..].iter());

	while let Ok(page) = Page::read(&mut data) {
		if md_pages.len() > 125_829_120 {
			return Err(LoftyError::TooMuchData);
		}

		if page.header_type == 1 {
			md_pages.extend(page.content.iter());
		} else {
			break;
		}
	}

	let mut md: HashMap<String, String> = HashMap::new();
	let mut pictures = Vec::new();

	let reader = &mut &md_pages[..];

	let vendor_len = reader.read_u32::<LittleEndian>()?;
	let mut vendor = vec![0; vendor_len as usize];
	reader.read_exact(&mut vendor)?;

	let vendor_str = match String::from_utf8(vendor) {
		Ok(v) => v,
		Err(_) => {
			return Err(LoftyError::InvalidData(
				"OGG file has an invalid vendor string",
			))
		},
	};

	let comments_total_len = reader.read_u32::<LittleEndian>()?;

	for _ in 0..comments_total_len {
		let comment_len = reader.read_u32::<LittleEndian>()?;

		let mut comment_bytes = vec![0; comment_len as usize];
		reader.read_exact(&mut comment_bytes)?;

		let comment = String::from_utf8(comment_bytes)?;

		let split: Vec<&str> = comment.splitn(2, '=').collect();

		if split[0] == "METADATA_BLOCK_PICTURE" {
			pictures.push(Picture::from_apic_bytes(split[1].as_bytes())?)
		} else {
			md.insert(split[0].to_string(), split[1].to_string());
		}
	}

	let vorbis_format = if header_sig == OPUSHEAD {
		OggFormat::Opus
	} else {
		OggFormat::Vorbis
	};

	Ok((vendor_str, pictures, md, vorbis_format))
}
