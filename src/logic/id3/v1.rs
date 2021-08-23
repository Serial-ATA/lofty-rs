use super::constants::GENRES;
use crate::{ItemKey, ItemValue, Result, Tag, TagItem, TagType};

use std::io::{Read, Seek, SeekFrom};

pub(crate) fn find_id3v1<R>(data: &mut R, read: bool) -> Result<(bool, Option<Tag>)>
where
	R: Read + Seek,
{
	let mut id3v1 = None;
	let mut exists = false;

	data.seek(SeekFrom::End(-128))?;

	let mut id3v1_header = [0; 3];
	data.read_exact(&mut id3v1_header)?;

	data.seek(SeekFrom::Current(-3))?;

	if &id3v1_header == b"TAG" {
		exists = true;

		if read {
			let mut id3v1_tag = [0; 128];
			data.read_exact(&mut id3v1_tag)?;

			data.seek(SeekFrom::End(-128))?;

			id3v1 = Some(parse_id3v1(id3v1_tag))
		}
	} else {
		// No ID3v1 tag found
		data.seek(SeekFrom::End(0))?;
	}

	Ok((exists, id3v1))
}

pub(crate) fn parse_id3v1(reader: [u8; 128]) -> Tag {
	let mut tag = Tag::new(TagType::Id3v1);

	if let Some(title) = decode_text(ItemKey::TrackTitle, &reader[3..33]) {
		tag.insert_item(title);
	}

	if let Some(artist) = decode_text(ItemKey::TrackArtist, &reader[33..63]) {
		tag.insert_item(artist);
	}

	if let Some(album) = decode_text(ItemKey::AlbumTitle, &reader[63..93]) {
		tag.insert_item(album);
	}

	if let Some(year) = decode_text(ItemKey::Year, &reader[93..97]) {
		tag.insert_item(year);
	}

	let range = if reader[122] == 0 {
		if let Ok(track) = String::from_utf8(vec![reader[123]]) {
			tag.insert_item(TagItem::new(ItemKey::TrackNumber, ItemValue::Text(track)));
		}

		97_usize..122
	} else {
		97..124
	};

	if let Some(comment) = decode_text(ItemKey::Comment, &reader[range]) {
		tag.insert_item(comment);
	}

	if reader[125] < GENRES.len() as u8 {
		tag.insert_item(TagItem::new(
			ItemKey::Genre,
			ItemValue::Text(GENRES[reader[125] as usize].to_string()),
		));
	}

	tag
}

fn decode_text(key: ItemKey, data: &[u8]) -> Option<TagItem> {
	let read = data
		.iter()
		.filter(|c| **c > 0x1f)
		.map(|c| *c as char)
		.collect::<String>();

	if read.is_empty() {
		None
	} else {
		Some(TagItem::new(key, ItemValue::Text(read)))
	}
}
