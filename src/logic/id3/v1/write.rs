use crate::error::Result;
use crate::types::item::{ItemKey, ItemValue, TagItem};
use crate::types::tag::Tag;

use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use byteorder::WriteBytesExt;

pub fn write_id3v1<W>(writer: &mut W, tag: &Tag) -> Result<()>
where
	W: Write + Read + Seek,
{
	let tag = encode(tag)?;

	// This will seek us to the writing position
	super::find_id3v1(writer, false)?;

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
