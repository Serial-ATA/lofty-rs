use super::constants::GENRES;
use crate::error::Result;
use crate::types::item::{ItemKey, ItemValue, TagItem};
use crate::types::tag::{Tag, TagType};

use byteorder::WriteBytesExt;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

pub(in crate::logic) fn find_id3v1<R>(data: &mut R, read: bool) -> Result<(bool, Option<Tag>)>
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

pub(in crate::logic) fn parse_id3v1(reader: [u8; 128]) -> Tag {
	let mut tag = Tag::new(TagType::Id3v1);

	let reader = &reader[3..];

	if let Some(title) = decode_text(ItemKey::TrackTitle, &reader[..30]) {
		tag.insert_item_unchecked(title);
	}

	if let Some(artist) = decode_text(ItemKey::TrackArtist, &reader[30..60]) {
		tag.insert_item_unchecked(artist);
	}

	if let Some(album) = decode_text(ItemKey::AlbumTitle, &reader[60..90]) {
		tag.insert_item_unchecked(album);
	}

	if let Some(year) = decode_text(ItemKey::Year, &reader[90..94]) {
		tag.insert_item_unchecked(year);
	}

	let range = if reader[119] == 0 && reader[122] != 0 {
		tag.insert_item_unchecked(TagItem::new(
			ItemKey::TrackNumber,
			ItemValue::UInt(u32::from(reader[122])),
		));

		94_usize..123
	} else {
		94..124
	};

	if let Some(comment) = decode_text(ItemKey::Comment, &reader[range]) {
		tag.insert_item_unchecked(comment);
	}

	if reader[124] < GENRES.len() as u8 {
		tag.insert_item_unchecked(TagItem::new(
			ItemKey::Genre,
			ItemValue::Text(GENRES[reader[125] as usize].to_string()),
		));
	}

	tag
}

fn decode_text(key: ItemKey, data: &[u8]) -> Option<TagItem> {
	let read = data
		.iter()
		.filter(|c| **c != 0)
		.map(|c| *c as char)
		.collect::<String>();

	if read.is_empty() {
		None
	} else {
		Some(TagItem::new(key, ItemValue::Text(read)))
	}
}

pub(in crate::logic) fn write_id3v1<W>(writer: &mut W, tag: &Tag) -> Result<()>
where
	W: Write + Read + Seek,
{
	let tag = encode(tag)?;

	// This will seek us to the writing position
	find_id3v1(writer, false)?;

	writer.write_all(&tag)?;

	Ok(())
}

fn encode(tag: &Tag) -> Result<Vec<u8>> {
	fn resize_string(item: Option<&TagItem>, size: usize) -> Result<Vec<u8>> {
		let mut cursor = Cursor::new(vec![0; size]);
		cursor.seek(SeekFrom::Start(0))?;

		if let Some(ItemValue::Text(text)) = item.map(TagItem::value) {
			if text.len() > size {
				cursor.write_all(text.split_at(size).0.as_bytes())?;
			} else {
				cursor.write_all(text.as_bytes())?;
			}
		}

		Ok(cursor.into_inner())
	}

	let mut writer = Vec::with_capacity(128);

	writer.write_all(&[b'T', b'A', b'G'])?;

	let title = resize_string(tag.get_item_ref(&ItemKey::TrackTitle), 30)?;
	writer.write_all(&*title)?;

	let artist = resize_string(tag.get_item_ref(&ItemKey::TrackArtist), 30)?;
	writer.write_all(&*artist)?;

	let album = resize_string(tag.get_item_ref(&ItemKey::AlbumTitle), 30)?;
	writer.write_all(&*album)?;

	let year = resize_string(tag.get_item_ref(&ItemKey::Year), 4)?;
	writer.write_all(&*year)?;

	let comment = resize_string(tag.get_item_ref(&ItemKey::Comment), 28)?;
	writer.write_all(&*comment)?;

	writer.write_u8(0)?;

	let item_to_byte = |key: &ItemKey, max: u8, empty: u8| {
		if let Some(track_number) = tag.get_item_ref(key) {
			match track_number.value() {
				ItemValue::Text(text) => {
					if let Ok(parsed) = text.parse::<u8>() {
						if parsed <= max {
							return parsed;
						}
					}

					empty
				},
				ItemValue::UInt(i) => {
					if *i <= u32::from(max) {
						*i as u8
					} else {
						empty
					}
				},
				ItemValue::UInt64(i) => {
					if *i <= u64::from(max) {
						*i as u8
					} else {
						empty
					}
				},
				ItemValue::Int(i) => {
					if i.is_positive() && *i <= i32::from(max) {
						*i as u8
					} else {
						empty
					}
				},
				ItemValue::Int64(i) => {
					if i.is_positive() && *i <= i64::from(max) {
						*i as u8
					} else {
						empty
					}
				},
				_ => empty,
			}
		} else {
			empty
		}
	};

	writer.write_u8(item_to_byte(&ItemKey::TrackNumber, 255, 0))?;
	writer.write_u8(item_to_byte(&ItemKey::Genre, 191, 255))?;

	Ok(writer)
}
