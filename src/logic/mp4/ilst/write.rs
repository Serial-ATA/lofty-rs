use crate::error::{LoftyError, Result};
use crate::logic::mp4::moov::Moov;
use crate::logic::mp4::read::nested_atom;
use crate::logic::mp4::read::verify_mp4;
use crate::picture::MimeType;
use crate::types::item::ItemValue;
use crate::types::picture::Picture;
use crate::types::tag::{Tag, TagType};

use std::convert::TryInto;
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use byteorder::{BigEndian, WriteBytesExt};

pub(in crate::logic) fn write_to(data: &mut File, tag: &Tag) -> Result<()> {
	if tag.tag_type() != &TagType::Mp4Atom {
		return Err(LoftyError::UnsupportedTag);
	}

	verify_mp4(data)?;

	let moov = Moov::find(data)?;
	let pos = data.seek(SeekFrom::Current(0))?;

	data.seek(SeekFrom::Start(0))?;

	let mut file_bytes = Vec::new();
	data.read_to_end(&mut file_bytes)?;

	let mut cursor = Cursor::new(file_bytes);
	cursor.seek(SeekFrom::Start(pos))?;

	let ilst = build_ilst(tag)?;
	let remove_tag = ilst.is_empty();

	let udta = nested_atom(&mut cursor, moov.len, "udta")?;

	// Nothing to do
	if remove_tag && udta.is_none() {
		return Ok(());
	}

	// Total size of new atoms
	let new_udta_size;
	// Size of the existing udta atom
	let mut existing_udta_size = 0;

	// ilst is nested in udta.meta, so we need to check what atoms actually exist
	if let Some(udta) = udta {
		if let Some(meta) = nested_atom(&mut cursor, udta.len, "meta")? {
			// Skip version and flags
			cursor.seek(SeekFrom::Current(4))?;
			let (replacement, range, existing_ilst_size) =
				if let Some(ilst_existing) = nested_atom(&mut cursor, meta.len - 4, "ilst")? {
					let ilst_existing_size = ilst_existing.len;

					let replacement = if remove_tag { Vec::new() } else { ilst };

					(
						replacement,
						ilst_existing.start as usize
							..(ilst_existing.start + ilst_existing.len) as usize,
						ilst_existing_size as u64,
					)
				} else {
					// Nothing to do
					if remove_tag {
						return Ok(());
					}

					let meta_end = (meta.start + meta.len) as usize;

					(ilst, meta_end..meta_end, 0)
				};

			existing_udta_size = udta.len;

			let new_meta_size = (meta.len - existing_ilst_size) + replacement.len() as u64;
			new_udta_size = (udta.len - meta.len) + new_meta_size;

			cursor.get_mut().splice(range, replacement);

			cursor.seek(SeekFrom::Start(meta.start))?;
			write_size(meta.start, new_meta_size, meta.extended, &mut cursor)?;

			cursor.seek(SeekFrom::Start(udta.start))?;
			write_size(udta.start, new_udta_size, udta.extended, &mut cursor)?;
		} else {
			// Nothing to do
			if remove_tag {
				return Ok(());
			}

			existing_udta_size = udta.len;

			let mut bytes = Cursor::new(vec![0, 0, 0, 0, b'm', b'e', b't', b'a']);

			write_size(0, ilst.len() as u64 + 8, false, &mut bytes)?;

			bytes.write_all(&ilst)?;
			let bytes = bytes.into_inner();

			new_udta_size = udta.len + bytes.len() as u64;

			cursor.seek(SeekFrom::Start(udta.start))?;
			write_size(udta.start, new_udta_size, udta.extended, &mut cursor)?;

			cursor
				.get_mut()
				.splice(udta.start as usize..udta.start as usize, bytes);
		}
	} else {
		let mut bytes = Cursor::new(vec![
			0, 0, 0, 0, b'u', b'd', b't', b'a', 0, 0, 0, 0, b'm', b'e', b't', b'a',
		]);

		// udta size
		write_size(0, ilst.len() as u64 + 8, false, &mut bytes)?;

		// meta size
		write_size(
			bytes.seek(SeekFrom::Current(0))?,
			ilst.len() as u64,
			false,
			&mut bytes,
		)?;

		bytes.seek(SeekFrom::End(0))?;
		bytes.write_all(&ilst)?;

		let bytes = bytes.into_inner();

		new_udta_size = bytes.len() as u64;

		cursor
			.get_mut()
			.splice((moov.start + 8) as usize..(moov.start + 8) as usize, bytes);
	}

	cursor.seek(SeekFrom::Start(moov.start))?;

	// Change the size of the moov atom
	write_size(
		moov.start,
		(moov.len - existing_udta_size) + new_udta_size,
		moov.extended,
		&mut cursor,
	)?;

	data.seek(SeekFrom::Start(0))?;
	data.set_len(0)?;
	data.write_all(&cursor.into_inner())?;

	Ok(())
}

fn write_size(start: u64, size: u64, extended: bool, writer: &mut Cursor<Vec<u8>>) -> Result<()> {
	if size > u64::from(u32::MAX) {
		// 0001 (identifier) ????????
		writer.write_u32::<BigEndian>(1)?;
		// Skip identifier
		writer.seek(SeekFrom::Current(4))?;

		let extended_size = size.to_be_bytes();
		let inner = writer.get_mut();

		if extended {
			// Overwrite existing extended size
			writer.write_u64::<BigEndian>(size)?;
		} else {
			for i in extended_size {
				inner.insert((start + 8 + u64::from(i)) as usize, i);
			}

			writer.seek(SeekFrom::Current(8))?;
		}
	} else {
		// ???? (identifier)
		writer.write_u32::<BigEndian>(size as u32)?;
		writer.seek(SeekFrom::Current(4))?;
	}

	Ok(())
}

fn build_ilst(tag: &Tag) -> Result<Vec<u8>> {
	if tag.item_count() == 0 && tag.picture_count() == 0 {
		return Ok(Vec::new());
	}

	let items = tag
		.items()
		.iter()
		.filter_map(|i| {
			let key = i.key().map_key(&TagType::Mp4Atom).unwrap();
			let valid_value = std::mem::discriminant(&ItemValue::SynchronizedText(Vec::new()))
				!= std::mem::discriminant(i.value())
				&& std::mem::discriminant(&ItemValue::Binary(Vec::new()))
					!= std::mem::discriminant(i.value());

			((key.chars().count() == 4 || key.starts_with("----")) && valid_value)
				.then(|| (key, i.value()))
		})
		.collect::<Vec<(&str, &ItemValue)>>();

	if items.is_empty() {
		return Ok(Vec::new());
	}

	let mut writer = Cursor::new(vec![0, 0, 0, 0, b'i', b'l', b's', b't']);
	writer.seek(SeekFrom::End(0))?;

	for (key, value) in items {
		let start = writer.seek(SeekFrom::Current(0))?;

		// Empty size, we get it later
		writer.write_all(&[0; 4])?;

		if key.starts_with("----") {
			write_freeform(key, &mut writer)?;
		} else {
			// "©" is 2 bytes, we only want to write the second one
			writer.write_all(&if key.starts_with('©') {
				let key_bytes = key.as_bytes();

				[key_bytes[1], key_bytes[2], key_bytes[3], key_bytes[4]]
			} else if key.len() > 4 {
				return Err(LoftyError::BadAtom(
					"Attempted to write an atom identifier bigger than 4 bytes",
				));
			} else {
				key.as_bytes().try_into().unwrap()
			})?;
		}

		write_item(value, &mut writer)?;

		let end = writer.seek(SeekFrom::Current(0))?;

		let size = end - start;

		writer.seek(SeekFrom::Start(start))?;

		write_size(start, size, false, &mut writer)?;

		writer.seek(SeekFrom::Start(end))?;
	}

	for pic in tag.pictures() {
		write_picture(pic, &mut writer)?;
	}

	let size = writer.get_ref().len();

	write_size(
		writer.seek(SeekFrom::Start(0))?,
		size as u64,
		false,
		&mut writer,
	)?;

	Ok(writer.into_inner())
}

fn write_freeform(freeform: &str, writer: &mut Cursor<Vec<u8>>) -> Result<()> {
	// ---- : ???? : ????
	let freeform_split = freeform.splitn(3, ':').collect::<Vec<&str>>();

	if freeform_split.len() != 3 {
		return Err(LoftyError::BadAtom(
			"Attempted to write an incomplete freeform identifier",
		));
	}

	// ----
	writer.write_all(freeform_split[0].as_bytes())?;

	// .... MEAN 0000 ????
	let mean = freeform_split[1];

	writer.write_u32::<BigEndian>((12 + mean.len()) as u32)?;
	writer.write_all(&[b'm', b'e', b'a', b'n', 0, 0, 0, 0])?;
	writer.write_all(mean.as_bytes())?;

	// .... NAME 0000 ????
	let name = freeform_split[2];

	writer.write_u32::<BigEndian>((12 + name.len()) as u32)?;
	writer.write_all(&[b'n', b'a', b'm', b'e', 0, 0, 0, 0])?;
	writer.write_all(name.as_bytes())?;

	Ok(())
}

fn write_item(value: &ItemValue, writer: &mut Cursor<Vec<u8>>) -> Result<()> {
	match value {
		ItemValue::Text(text) => write_data(1, text.as_bytes(), writer),
		ItemValue::Locator(locator) => write_data(2, locator.as_bytes(), writer),
		ItemValue::UInt(uint) => write_data(22, uint.to_be_bytes().as_ref(), writer),
		ItemValue::UInt64(uint64) => write_data(78, uint64.to_be_bytes().as_ref(), writer),
		ItemValue::Int(int) => write_data(21, int.to_be_bytes().as_ref(), writer),
		ItemValue::Int64(int64) => write_data(74, int64.to_be_bytes().as_ref(), writer),
		_ => unreachable!(),
	}
}

fn write_picture(picture: &Picture, writer: &mut Cursor<Vec<u8>>) -> Result<()> {
	match picture.mime_type {
		// GIF is deprecated
		MimeType::Gif => write_data(12, &picture.data, writer),
		MimeType::Jpeg => write_data(13, &picture.data, writer),
		MimeType::Png => write_data(14, &picture.data, writer),
		MimeType::Bmp => write_data(27, &picture.data, writer),
		// We'll assume implicit (0) was the intended type
		MimeType::None => write_data(0, &picture.data, writer),
		_ => Err(LoftyError::BadAtom(
			"Attempted to write an unsupported picture format",
		)),
	}
}

fn write_data(flags: u8, data: &[u8], writer: &mut Cursor<Vec<u8>>) -> Result<()> {
	// .... DATA (flags) 0000 (data)
	let size = 16_u64 + data.len() as u64;

	writer.write_all(&[0, 0, 0, 0, b'd', b'a', b't', b'a'])?;
	write_size(writer.seek(SeekFrom::Current(-8))?, size, false, writer)?;

	// Version
	writer.write_u8(0)?;

	writer.write_all(&[0, 0, flags])?;
	writer.write_all(&[0; 4])?;
	writer.write_all(data)?;

	Ok(())
}
