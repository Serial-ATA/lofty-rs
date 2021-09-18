use super::{Id3v2Frame, LanguageSpecificFrame};
use crate::error::Result;
use crate::logic::id3::v2::util::text_utils::{decode_text, TextEncoding};
use crate::logic::id3::v2::Id3v2Version;
use crate::types::picture::Picture;
use crate::{ItemKey, ItemValue, LoftyError, TagItem, TagType};

use std::io::Read;

use byteorder::ReadBytesExt;

pub(crate) enum FrameContent {
	Picture(Picture),
	// For values that only apply to an Id3v2Frame
	Item(TagItem),
}

pub(crate) fn parse_content(
	content: &mut &[u8],
	id: &str,
	version: Id3v2Version,
) -> Result<FrameContent> {
	Ok(match id {
		// The ID was previously upgraded, but the content remains unchanged, so version is necessary
		"APIC" => FrameContent::Picture(Picture::from_apic_bytes(content, version)?),
		"TXXX" => FrameContent::Item(parse_user_defined(content, false)?),
		"WXXX" => FrameContent::Item(parse_user_defined(content, true)?),
		"COMM" | "USLT" => FrameContent::Item(parse_text_language(id, content)?),
		"SYLT" => FrameContent::Item({
			TagItem::new(
				ItemKey::Id3v2Specific(Id3v2Frame::SyncText),
				ItemValue::Binary(content.to_vec()),
			)
		}),
		"GEOB" => FrameContent::Item({
			TagItem::new(
				ItemKey::Id3v2Specific(Id3v2Frame::EncapsulatedObject),
				ItemValue::Binary(content.to_vec()),
			)
		}),
		_ if id.starts_with('T') => FrameContent::Item(parse_text(id, content)?),
		_ if id.starts_with('W') => FrameContent::Item(parse_link(id, content)?),
		_ => FrameContent::Item(TagItem::new(
			ItemKey::from_key(&TagType::Id3v2, id)
				.unwrap_or_else(|| ItemKey::Unknown(id.to_string())),
			ItemValue::Binary(content.to_vec()),
		)),
	})
}

// There are 2 possibilities for the frame's content: text or link.
fn parse_user_defined(content: &mut &[u8], link: bool) -> Result<TagItem> {
	if content.len() < 2 {
		return Err(LoftyError::BadFrameLength);
	}

	let encoding = match TextEncoding::from_u8(content.read_u8()?) {
		None => return Err(LoftyError::TextDecode("Found invalid encoding")),
		Some(e) => e,
	};

	let description = decode_text(content, encoding, true)?.unwrap_or_else(String::new);

	Ok(if link {
		let content =
			decode_text(content, TextEncoding::Latin1, false)?.unwrap_or_else(String::new);

		TagItem::new(
			ItemKey::Id3v2Specific(Id3v2Frame::UserURL(encoding, description)),
			ItemValue::Locator(content),
		)
	} else {
		let content = decode_text(content, encoding, false)?.unwrap_or_else(String::new);

		TagItem::new(
			ItemKey::Id3v2Specific(Id3v2Frame::UserText(encoding, description)),
			ItemValue::Text(content),
		)
	})
}

fn parse_text_language(id: &str, content: &mut &[u8]) -> Result<TagItem> {
	if content.len() < 5 {
		return Err(LoftyError::BadFrameLength);
	}

	let encoding = match TextEncoding::from_u8(content.read_u8()?) {
		None => return Err(LoftyError::TextDecode("Found invalid encoding")),
		Some(e) => e,
	};

	let mut lang = [0; 3];
	content.read_exact(&mut lang)?;

	let lang = std::str::from_utf8(&lang)
		.map_err(|_| LoftyError::TextDecode("Unable to decode language string"))?;

	let description = decode_text(content, encoding, true)?;
	let content = decode_text(content, encoding, false)?.unwrap_or_else(String::new);

	let information = LanguageSpecificFrame {
		encoding,
		language: lang.to_string(),
		description,
	};

	let item_key = match id {
		"COMM" => ItemKey::Id3v2Specific(Id3v2Frame::Comment(information)),
		"USLT" => ItemKey::Id3v2Specific(Id3v2Frame::UnSyncText(information)),
		_ => unreachable!(),
	};

	Ok(TagItem::new(item_key, ItemValue::Text(content)))
}

fn parse_text(id: &str, content: &mut &[u8]) -> Result<TagItem> {
	let encoding = match TextEncoding::from_u8(content.read_u8()?) {
		None => return Err(LoftyError::TextDecode("Found invalid encoding")),
		Some(e) => e,
	};

	let text = decode_text(content, encoding, false)?.unwrap_or_else(String::new);

	let key = ItemKey::from_key(&TagType::Id3v2, id)
		.unwrap_or_else(|| ItemKey::Id3v2Specific(Id3v2Frame::Text(id.to_string(), encoding)));

	Ok(TagItem::new(key, ItemValue::Text(text)))
}

fn parse_link(id: &str, content: &mut &[u8]) -> Result<TagItem> {
	let link = decode_text(content, TextEncoding::Latin1, false)?.unwrap_or_else(String::new);

	let key = ItemKey::from_key(&TagType::Id3v2, id)
		.unwrap_or_else(|| ItemKey::Id3v2Specific(Id3v2Frame::URL(id.to_string())));

	Ok(TagItem::new(key, ItemValue::Locator(link)))
}
