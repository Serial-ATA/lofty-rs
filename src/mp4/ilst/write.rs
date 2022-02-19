use super::{AtomDataRef, IlstRef};
use crate::error::{FileEncodingError, Result};
use crate::mp4::ilst::{AtomIdentRef, AtomRef};
use crate::mp4::moov::Moov;
use crate::mp4::read::{nested_atom, verify_mp4};
use crate::types::file::FileType;
use crate::types::picture::{MimeType, Picture};

use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use byteorder::{BigEndian, WriteBytesExt};

pub(in crate) fn write_to(data: &mut File, tag: &mut IlstRef<'_>) -> Result<()> {
	verify_mp4(data)?;

	let moov = Moov::find(data)?;
	let pos = data.seek(SeekFrom::Current(0))?;

	data.seek(SeekFrom::Start(0))?;

	let mut file_bytes = Vec::new();
	data.read_to_end(&mut file_bytes)?;

	let mut cursor = Cursor::new(file_bytes);
	cursor.seek(SeekFrom::Start(pos))?;

	let ilst = build_ilst(&mut tag.atoms)?;
	let remove_tag = ilst.is_empty();

	let udta = nested_atom(&mut cursor, moov.len, b"udta")?;

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
		if let Some(meta) = nested_atom(&mut cursor, udta.len, b"meta")? {
			// Skip 4 bytes
			// Version (1)
			// Flags   (3)
			cursor.seek(SeekFrom::Current(4))?;

			let replacement;
			let range;
			let existing_ilst_size;

			let existing_ilst = nested_atom(&mut cursor, meta.len - 4, b"ilst")?;

			match existing_ilst {
				Some(existing) => {
					replacement = if remove_tag { Vec::new() } else { ilst };

					range = existing.start as usize..(existing.start + existing.len) as usize;
					existing_ilst_size = existing.len as u64;
				},
				None => {
					// Nothing to do
					if remove_tag {
						return Ok(());
					}

					let meta_end = (meta.start + meta.len) as usize;

					replacement = ilst;
					range = meta_end..meta_end;
					existing_ilst_size = 0;
				},
			}

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

			// `meta` + `ilst`
			let capacity = 8 + ilst.len();
			let buf = Vec::with_capacity(capacity);

			let mut bytes = Cursor::new(buf);
			bytes.write_all(&[0, 0, 0, 0, b'm', b'e', b't', b'a'])?;

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
		// `udta` + `meta` + `ilst`
		let capacity = 16 + ilst.len();
		let buf = Vec::with_capacity(capacity);

		let mut bytes = Cursor::new(buf);
		bytes.write_all(&[
			0, 0, 0, 0, b'u', b'd', b't', b'a', 0, 0, 0, 0, b'm', b'e', b't', b'a',
		])?;

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

pub(super) fn build_ilst(atoms: &mut dyn Iterator<Item = AtomRef<'_>>) -> Result<Vec<u8>> {
	let mut peek = atoms.peekable();

	if peek.peek().is_none() {
		return Ok(Vec::new());
	}

	let mut writer = Cursor::new(vec![0, 0, 0, 0, b'i', b'l', b's', b't']);
	writer.seek(SeekFrom::End(0))?;

	for atom in peek {
		let start = writer.seek(SeekFrom::Current(0))?;

		// Empty size, we get it later
		writer.write_all(&[0; 4])?;

		match atom.ident {
			AtomIdentRef::Fourcc(ref fourcc) => writer.write_all(fourcc)?,
			AtomIdentRef::Freeform { mean, name } => write_freeform(mean, name, &mut writer)?,
		}

		write_atom_data(&atom.data, &mut writer)?;

		let end = writer.seek(SeekFrom::Current(0))?;

		let size = end - start;

		writer.seek(SeekFrom::Start(start))?;

		write_size(start, size, false, &mut writer)?;

		writer.seek(SeekFrom::Start(end))?;
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

fn write_freeform(mean: &str, name: &str, writer: &mut Cursor<Vec<u8>>) -> Result<()> {
	// ---- : ???? : ????

	// ----
	writer.write_all(b"----")?;

	// .... MEAN 0000 ????
	writer.write_u32::<BigEndian>((12 + mean.len()) as u32)?;
	writer.write_all(&[b'm', b'e', b'a', b'n', 0, 0, 0, 0])?;
	writer.write_all(mean.as_bytes())?;

	// .... NAME 0000 ????
	writer.write_u32::<BigEndian>((12 + name.len()) as u32)?;
	writer.write_all(&[b'n', b'a', b'm', b'e', 0, 0, 0, 0])?;
	writer.write_all(name.as_bytes())?;

	Ok(())
}

fn write_atom_data(value: &AtomDataRef<'_>, writer: &mut Cursor<Vec<u8>>) -> Result<()> {
	match value {
		AtomDataRef::UTF8(text) => write_data(1, text.as_bytes(), writer),
		AtomDataRef::UTF16(text) => write_data(2, text.as_bytes(), writer),
		AtomDataRef::Picture(pic) => write_picture(pic, writer),
		AtomDataRef::SignedInteger(int) => write_signed_int(*int, writer),
		AtomDataRef::UnsignedInteger(uint) => write_unsigned_int(*uint, writer),
		AtomDataRef::Unknown { code, data } => write_data(*code, data, writer),
	}
}

fn write_signed_int(int: i32, writer: &mut Cursor<Vec<u8>>) -> Result<()> {
	let start_pos = int.trailing_zeros() as usize;
	write_int(21, int.to_be_bytes(), start_pos, writer)
}

fn write_unsigned_int(uint: u32, writer: &mut Cursor<Vec<u8>>) -> Result<()> {
	let start_pos = uint.trailing_zeros() as usize;
	write_int(22, uint.to_be_bytes(), start_pos, writer)
}

fn write_int(
	flags: u32,
	bytes: [u8; 4],
	mut start_pos: usize,
	writer: &mut Cursor<Vec<u8>>,
) -> Result<()> {
	if start_pos == 4 {
		start_pos = 0;
	}
	write_data(flags, &bytes[start_pos..], writer)
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
		_ => Err(FileEncodingError::new(
			FileType::MP4,
			"Attempted to write an unsupported picture format",
		)
		.into()),
	}
}

fn write_data(flags: u32, data: &[u8], writer: &mut Cursor<Vec<u8>>) -> Result<()> {
	if flags > 16_777_215 {
		return Err(FileEncodingError::new(
			FileType::MP4,
			"Attempted to write a code that cannot fit in 24 bits",
		)
		.into());
	}

	// .... DATA (version = 0) (flags) (locale = 0000) (data)
	let size = 16_u64 + data.len() as u64;

	writer.write_all(&[0, 0, 0, 0, b'd', b'a', b't', b'a'])?;
	write_size(writer.seek(SeekFrom::Current(-8))?, size, false, writer)?;

	// Version
	writer.write_u8(0)?;

	writer.write_uint::<BigEndian>(u64::from(flags), 3)?;

	// Locale
	writer.write_all(&[0; 4])?;
	writer.write_all(data)?;

	Ok(())
}
