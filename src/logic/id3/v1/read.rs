use super::constants::GENRES;
use crate::types::item::{ItemKey, ItemValue, TagItem};
use crate::types::tag::{Tag, TagType};

pub fn parse_id3v1(reader: [u8; 128]) -> Tag {
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
