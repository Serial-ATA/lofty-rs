use crate::error::{LoftyError, Result};
use crate::logic::id3::v2::util::text_utils::utf16_decode;
use crate::logic::id3::v2::TextEncoding;
use crate::logic::mp4::atom::Atom;
use crate::logic::mp4::read::skip_unneeded;
use crate::types::item::ItemKey;
use crate::types::picture::{MimeType, Picture, PictureInformation, PictureType};
use crate::types::tag::{ItemValue, Tag, TagItem, TagType};

use std::borrow::Cow;
use std::io::{Cursor, Read, Seek, SeekFrom};

use byteorder::{BigEndian, ReadBytesExt};

pub(crate) fn parse_ilst<R>(data: &mut R, len: u64) -> Result<Option<Tag>>
where
	R: Read + Seek,
{
	let mut contents = vec![0; len as usize];
	data.read_exact(&mut contents)?;

	let mut cursor = Cursor::new(contents);

	let mut tag = Tag::new(TagType::Mp4Atom);

	while let Ok(atom) = Atom::read(&mut cursor) {
		// Safe to unwrap here since ItemKey::Unknown exists
		let key = match &*atom.ident {
			"free" | "skip" => {
				skip_unneeded(&mut cursor, atom.extended, atom.len)?;
				continue;
			},
			"covr" => {
				let (mime_type, picture) = match parse_data(&mut cursor)? {
					(ItemValue::Binary(picture), 13) => (MimeType::Jpeg, picture),
					(ItemValue::Binary(picture), 14) => (MimeType::Png, picture),
					(ItemValue::Binary(picture), 27) => (MimeType::Bmp, picture),
					// GIF is deprecated
					(ItemValue::Binary(picture), 12) => (MimeType::Gif, picture),
					// Type 0 is implicit
					(ItemValue::Binary(picture), 0) => (MimeType::None, picture),
					_ => return Err(LoftyError::BadAtom("\"covr\" atom has an unknown type")),
				};

				tag.push_picture(Picture {
					pic_type: PictureType::Other,
					text_encoding: TextEncoding::UTF8,
					mime_type,
					description: None,
					information: PictureInformation {
						width: 0,
						height: 0,
						color_depth: 0,
						num_colors: 0,
					},
					data: Cow::from(picture),
				});

				continue;
			},
			"----" => ItemKey::from_key(&TagType::Mp4Atom, &*parse_freeform(&mut cursor)?),
			other => ItemKey::from_key(&TagType::Mp4Atom, other),
		}
		.unwrap();

		let data = parse_data(&mut cursor)?.0;

		match key {
			ItemKey::TrackNumber | ItemKey::DiscNumber => {
				if let ItemValue::Binary(pair) = data {
					let pair = &mut &pair[2..6];

					let number = u32::from(pair.read_u16::<BigEndian>()?);
					let total = u32::from(pair.read_u16::<BigEndian>()?);

					if total == 0 {
						match key {
							ItemKey::TrackNumber => tag.insert_item_unchecked(TagItem::new(
								ItemKey::TrackTotal,
								ItemValue::UInt(total),
							)),
							ItemKey::DiscNumber => tag.insert_item_unchecked(TagItem::new(
								ItemKey::DiscTotal,
								ItemValue::UInt(total),
							)),
							_ => unreachable!(),
						}
					}

					if number == 0 {
						tag.insert_item_unchecked(TagItem::new(key, ItemValue::UInt(number)))
					}
				} else {
					return Err(LoftyError::BadAtom(
						"Expected atom data to include integer pair",
					));
				}
			},
			_ => tag.insert_item_unchecked(TagItem::new(key, data)),
		}
	}

	Ok(Some(tag))
}

fn parse_data<R>(data: &mut R) -> Result<(ItemValue, u32)>
where
	R: Read + Seek,
{
	let atom = Atom::read(data)?;

	if atom.ident != "data" {
		return Err(LoftyError::BadAtom("Expected atom \"data\" to follow name"));
	}

	// We don't care about the version
	let _version = data.read_u8()?;

	let mut flags = [0; 3];
	data.read_exact(&mut flags)?;

	let flags = u32::from_be_bytes([0, flags[0], flags[1], flags[2]]);

	// We don't care about the locale
	data.seek(SeekFrom::Current(4))?;

	let mut content = vec![0; (atom.len - 16) as usize];
	data.read_exact(&mut content)?;

	// https://developer.apple.com/library/archive/documentation/QuickTime/QTFF/Metadata/Metadata.html#//apple_ref/doc/uid/TP40000939-CH1-SW35
	let value = match flags {
		1 => ItemValue::Text(String::from_utf8(content)?),
		2 => ItemValue::Text(utf16_decode(&*content, u16::from_be_bytes)?),
		15 => ItemValue::Locator(String::from_utf8(content)?),
		22 | 76 | 77 | 78 => parse_uint(&*content)?,
		21 | 66 | 67 | 74 => parse_int(&*content)?,
		_ => ItemValue::Binary(content),
	};

	Ok((value, flags))
}

fn parse_uint(bytes: &[u8]) -> Result<ItemValue> {
	Ok(match bytes.len() {
		1 => ItemValue::UInt(u32::from(bytes[0])),
		2 => ItemValue::UInt(u32::from(u16::from_be_bytes([bytes[0], bytes[1]]))),
		3 => ItemValue::UInt(u32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]])),
		4 => ItemValue::UInt(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])),
		8 => ItemValue::UInt64(u64::from_be_bytes([
			bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
		])),
		_ => {
			return Err(LoftyError::BadAtom(
				"Unexpected atom size for type \"BE unsigned integer\"",
			))
		},
	})
}

fn parse_int(bytes: &[u8]) -> Result<ItemValue> {
	Ok(match bytes.len() {
		1 => ItemValue::Int(i32::from(bytes[0])),
		2 => ItemValue::Int(i32::from(i16::from_be_bytes([bytes[0], bytes[1]]))),
		3 => ItemValue::Int(i32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]]) as i32),
		4 => ItemValue::Int(i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as i32),
		8 => ItemValue::Int64(i64::from_be_bytes([
			bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
		])),
		_ => {
			return Err(LoftyError::BadAtom(
				"Unexpected atom size for type \"BE signed integer\"",
			))
		},
	})
}

fn parse_freeform<R>(data: &mut R) -> Result<String>
where
	R: Read + Seek,
{
	let mut freeform = String::new();
	freeform.push_str("----:");

	freeform_chunk(data, "mean", &mut freeform)?;
	freeform_chunk(data, "name", &mut freeform)?;

	Ok(freeform)
}

fn freeform_chunk<R>(data: &mut R, name: &str, freeform: &mut String) -> Result<()>
where
	R: Read + Seek,
{
	let atom = Atom::read(data)?;

	if atom.ident != name {
		return Err(LoftyError::BadAtom(
			"Found freeform identifier \"----\" with no trailing \"mean\" or \"name\" atoms",
		));
	}

	let mut content = vec![0; atom.len as usize];
	data.read_exact(&mut content)?;

	freeform.push_str(std::str::from_utf8(&*content).map_err(|_| {
		LoftyError::BadAtom("Found a non UTF-8 string while reading freeform identifier")
	})?);

	Ok(())
}
