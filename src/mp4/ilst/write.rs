use super::r#ref::IlstRef;
use crate::error::{FileEncodingError, Result};
use crate::file::FileType;
use crate::macros::{err, try_vec};
use crate::mp4::atom_info::{AtomIdent, AtomInfo, ATOM_HEADER_LEN, FOURCC_LEN, IDENTIFIER_LEN};
use crate::mp4::ilst::r#ref::AtomRef;
use crate::mp4::moov::Moov;
use crate::mp4::read::{atom_tree, meta_is_full, nested_atom, verify_mp4, AtomReader};
use crate::mp4::AtomData;
use crate::picture::{MimeType, Picture};

use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use byteorder::{BigEndian, WriteBytesExt};

// A "full" atom is a traditional length + identifier, followed by a version (1) and flags (3)
const FULL_ATOM_SIZE: u64 = ATOM_HEADER_LEN + 4;
const HDLR_SIZE: u64 = ATOM_HEADER_LEN + 25;

pub(crate) fn write_to<'a, I: 'a>(data: &mut File, tag: &mut IlstRef<'a, I>) -> Result<()>
where
	I: IntoIterator<Item = &'a AtomData>,
{
	let mut reader = AtomReader::new(data)?;

	verify_mp4(&mut reader)?;

	let moov = Moov::find(&mut reader)?;
	let pos = reader.stream_position()?;

	reader.rewind()?;

	let mut file_bytes = Vec::new();
	reader.read_to_end(&mut file_bytes)?;

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
	let mut new_udta_size;
	// Size of the existing udta atom
	let mut existing_udta_size = 0;

	// ilst is nested in udta.meta, so we need to check what atoms actually exist
	if let Some(udta) = udta {
		existing_udta_size = udta.len;
		new_udta_size = existing_udta_size;

		let meta = nested_atom(&mut cursor, udta.len, b"meta")?;
		match meta {
			Some(meta) => {
				// We may encounter a non-full `meta` atom
				meta_is_full(&mut cursor)?;

				// We can use the existing `udta` and `meta` atoms
				save_to_existing(
					&mut cursor,
					(meta, udta),
					&mut new_udta_size,
					ilst,
					remove_tag,
				)?
			},
			// Nothing to do
			None if remove_tag => return Ok(()),
			// We have to create the `meta` atom
			None => {
				existing_udta_size = udta.len;

				// `meta` + `ilst`
				let capacity = FULL_ATOM_SIZE as usize + ilst.len();
				let buf = Vec::with_capacity(capacity);

				let mut bytes = Cursor::new(buf);
				create_meta(&mut bytes, &ilst)?;

				let bytes = bytes.into_inner();

				new_udta_size = udta.len + bytes.len() as u64;

				cursor.seek(SeekFrom::Start(udta.start))?;
				write_size(udta.start, new_udta_size, udta.extended, &mut cursor)?;

				// We'll put the new `meta` atom right at the start of `udta`
				let meta_start_pos = (udta.start + ATOM_HEADER_LEN) as usize;
				cursor
					.get_mut()
					.splice(meta_start_pos..meta_start_pos, bytes);
			},
		}
	} else {
		// We have to create the `udta` atom
		let bytes = create_udta(&ilst)?;
		new_udta_size = bytes.len() as u64;

		// We'll put the new `udta` atom right at the start of `moov`
		let udta_pos = (moov.start + ATOM_HEADER_LEN) as usize;
		cursor.get_mut().splice(udta_pos..udta_pos, bytes);
	}

	cursor.seek(SeekFrom::Start(moov.start))?;

	// Change the size of the moov atom
	write_size(
		moov.start,
		(moov.len - existing_udta_size) + new_udta_size,
		moov.extended,
		&mut cursor,
	)?;

	let data = reader.into_inner();

	data.rewind()?;
	data.set_len(0)?;
	data.write_all(&cursor.into_inner())?;

	Ok(())
}

fn save_to_existing(
	cursor: &mut Cursor<Vec<u8>>,
	(meta, udta): (AtomInfo, AtomInfo),
	new_udta_size: &mut u64,
	ilst: Vec<u8>,
	remove_tag: bool,
) -> Result<()> {
	let replacement;
	let range;

	let (ilst_idx, tree) = atom_tree(cursor, meta.len - ATOM_HEADER_LEN, b"ilst")?;

	if tree.is_empty() {
		// Nothing to do
		if remove_tag {
			return Ok(());
		}

		let meta_end = (meta.start + meta.len) as usize;

		replacement = ilst;
		range = meta_end..meta_end;
	} else {
		let existing_ilst = &tree[ilst_idx];
		let existing_ilst_size = existing_ilst.len;

		let mut range_start = existing_ilst.start;
		let range_end = existing_ilst.start + existing_ilst_size;

		if remove_tag {
			// We just need to strip out the `ilst` atom

			replacement = Vec::new();
			range = range_start as usize..range_end as usize;
		} else {
			// Check for some padding atoms we can utilize
			let mut available_space = existing_ilst_size;

			// Check for one directly before the `ilst` atom
			if ilst_idx > 0 {
				let previous_atom = &tree[ilst_idx - 1];

				if previous_atom.ident == AtomIdent::Fourcc(*b"free") {
					range_start = previous_atom.start;
					available_space += previous_atom.len;
				}
			}

			// And after
			if ilst_idx != tree.len() - 1 {
				let next_atom = &tree[ilst_idx + 1];

				if next_atom.ident == AtomIdent::Fourcc(*b"free") {
					available_space += next_atom.len;
				}
			}

			let ilst_len = ilst.len() as u64;

			// Check if we have enough padding to fit the `ilst` atom and a new `free` atom
			if available_space > ilst_len && available_space - ilst_len > 8 {
				// We have enough space to make use of the padding

				let remaining_space = available_space - ilst_len;
				if remaining_space > u64::from(u32::MAX) {
					err!(TooMuchData);
				}

				let remaining_space = remaining_space as u32;

				cursor.seek(SeekFrom::Start(range_start))?;
				cursor.write_all(&ilst)?;

				// Write the remaining padding
				cursor.write_u32::<BigEndian>(remaining_space)?;
				cursor.write_all(b"free")?;
				cursor
					.write_all(&try_vec![1; (remaining_space - ATOM_HEADER_LEN as u32) as usize])?;

				return Ok(());
			}

			replacement = ilst;
			range = range_start as usize..range_end as usize;
		}
	}

	let new_meta_size = (meta.len - range.len() as u64) + replacement.len() as u64;

	// Replace the `ilst` atom
	cursor.get_mut().splice(range, replacement);

	if new_meta_size != meta.len {
		// We need to change the `meta` and `udta` atom sizes

		*new_udta_size = (udta.len - meta.len) + new_meta_size;

		cursor.seek(SeekFrom::Start(meta.start))?;
		write_size(meta.start, new_meta_size, meta.extended, cursor)?;

		cursor.seek(SeekFrom::Start(udta.start))?;
		write_size(udta.start, *new_udta_size, udta.extended, cursor)?;
	}

	Ok(())
}

fn create_udta(ilst: &[u8]) -> Result<Vec<u8>> {
	// `udta` + `meta` + `hdlr` + `ilst`
	let capacity = ATOM_HEADER_LEN + FULL_ATOM_SIZE + HDLR_SIZE + ilst.len() as u64;
	let buf = Vec::with_capacity(capacity as usize);

	let mut bytes = Cursor::new(buf);
	bytes.write_all(&[0, 0, 0, 0, b'u', b'd', b't', b'a'])?;

	create_meta(&mut bytes, ilst)?;

	// `udta` size
	bytes.rewind()?;
	write_size(0, bytes.get_ref().len() as u64, false, &mut bytes)?;

	Ok(bytes.into_inner())
}

fn create_meta(cursor: &mut Cursor<Vec<u8>>, ilst: &[u8]) -> Result<()> {
	let start = cursor.stream_position()?;
	// meta atom
	cursor.write_all(&[0, 0, 0, 0, b'm', b'e', b't', b'a', 0, 0, 0, 0])?;

	// hdlr atom
	cursor.write_u32::<BigEndian>(0)?;
	cursor.write_all(b"hdlr")?;
	cursor.write_u64::<BigEndian>(0)?;
	cursor.write_all(b"mdirappl")?;
	cursor.write_all(&[0, 0, 0, 0, 0, 0, 0, 0, 0])?;

	cursor.seek(SeekFrom::Start(start))?;

	let meta_size = FULL_ATOM_SIZE + HDLR_SIZE + ilst.len() as u64;
	write_size(start, meta_size, false, cursor)?;

	// Seek to `hdlr` size
	let hdlr_size_pos = cursor.seek(SeekFrom::Current(4))?;
	write_size(hdlr_size_pos, HDLR_SIZE, false, cursor)?;

	cursor.seek(SeekFrom::End(0))?;
	cursor.write_all(ilst)?;

	Ok(())
}

fn write_size(start: u64, size: u64, extended: bool, writer: &mut Cursor<Vec<u8>>) -> Result<()> {
	if size > u64::from(u32::MAX) {
		// 0001 (identifier) ????????
		writer.write_u32::<BigEndian>(1)?;
		// Skip identifier
		writer.seek(SeekFrom::Current(IDENTIFIER_LEN as i64))?;

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
		writer.seek(SeekFrom::Current(IDENTIFIER_LEN as i64))?;
	}

	Ok(())
}

pub(super) fn build_ilst<'a, I: 'a>(
	atoms: &mut dyn Iterator<Item = AtomRef<'a, I>>,
) -> Result<Vec<u8>>
where
	I: IntoIterator<Item = &'a AtomData>,
{
	let mut peek = atoms.peekable();

	if peek.peek().is_none() {
		return Ok(Vec::new());
	}

	let mut writer = Cursor::new(vec![0, 0, 0, 0, b'i', b'l', b's', b't']);
	writer.seek(SeekFrom::End(0))?;

	for atom in peek {
		let start = writer.stream_position()?;

		// Empty size, we get it later
		writer.write_all(&[0; FOURCC_LEN as usize])?;

		match atom.ident {
			AtomIdent::Fourcc(ref fourcc) => writer.write_all(fourcc)?,
			AtomIdent::Freeform { mean, name } => write_freeform(&mean, &name, &mut writer)?,
		}

		write_atom_data(atom.data, &mut writer)?;

		let end = writer.stream_position()?;

		let size = end - start;

		writer.seek(SeekFrom::Start(start))?;

		write_size(start, size, false, &mut writer)?;

		writer.seek(SeekFrom::Start(end))?;
	}

	let size = writer.get_ref().len();

	writer.rewind()?;

	write_size(0, size as u64, false, &mut writer)?;

	Ok(writer.into_inner())
}

fn write_freeform(mean: &str, name: &str, writer: &mut Cursor<Vec<u8>>) -> Result<()> {
	// ---- : ???? : ????

	// ----
	writer.write_all(b"----")?;

	// .... MEAN 0000 ????
	writer.write_u32::<BigEndian>((FULL_ATOM_SIZE + mean.len() as u64) as u32)?;
	writer.write_all(&[b'm', b'e', b'a', b'n', 0, 0, 0, 0])?;
	writer.write_all(mean.as_bytes())?;

	// .... NAME 0000 ????
	writer.write_u32::<BigEndian>((FULL_ATOM_SIZE + name.len() as u64) as u32)?;
	writer.write_all(&[b'n', b'a', b'm', b'e', 0, 0, 0, 0])?;
	writer.write_all(name.as_bytes())?;

	Ok(())
}

fn write_atom_data<'a, I: 'a>(data: I, writer: &mut Cursor<Vec<u8>>) -> Result<()>
where
	I: IntoIterator<Item = &'a AtomData>,
{
	for value in data {
		match value {
			AtomData::UTF8(text) => write_data(1, text.as_bytes(), writer)?,
			AtomData::UTF16(text) => write_data(2, text.as_bytes(), writer)?,
			AtomData::Picture(ref pic) => write_picture(pic, writer)?,
			AtomData::SignedInteger(int) => write_signed_int(*int, writer)?,
			AtomData::UnsignedInteger(uint) => write_unsigned_int(*uint, writer)?,
			AtomData::Bool(b) => write_signed_int(i32::from(*b), writer)?,
			AtomData::Unknown { code, ref data } => write_data(*code, data, writer)?,
		};
	}

	Ok(())
}

fn write_signed_int(int: i32, writer: &mut Cursor<Vec<u8>>) -> Result<()> {
	write_int(21, int.to_be_bytes(), 4, writer)
}

fn bytes_to_occupy_uint(uint: u32) -> usize {
	if uint == 0 {
		return 1;
	}

	let ret = 4 - (uint.to_le().leading_zeros() >> 3) as usize;
	if ret == 3 {
		return 4;
	}
	ret
}

fn write_unsigned_int(uint: u32, writer: &mut Cursor<Vec<u8>>) -> Result<()> {
	let bytes_needed = bytes_to_occupy_uint(uint);
	write_int(22, uint.to_be_bytes(), bytes_needed, writer)
}

fn write_int(
	flags: u32,
	bytes: [u8; 4],
	bytes_needed: usize,
	writer: &mut Cursor<Vec<u8>>,
) -> Result<()> {
	debug_assert!(bytes_needed != 0);
	write_data(flags, &bytes[4 - bytes_needed..], writer)
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
			FileType::Mp4,
			"Attempted to write an unsupported picture format",
		)
		.into()),
	}
}

fn write_data(flags: u32, data: &[u8], writer: &mut Cursor<Vec<u8>>) -> Result<()> {
	if flags > 16_777_215 {
		return Err(FileEncodingError::new(
			FileType::Mp4,
			"Attempted to write a code that cannot fit in 24 bits",
		)
		.into());
	}

	// .... DATA (version = 0) (flags) (locale = 0000) (data)
	let size = FULL_ATOM_SIZE + 4 + data.len() as u64;

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

#[cfg(test)]
mod tests {
	use crate::mp4::ilst::write::bytes_to_occupy_uint;

	macro_rules! int_test {
		(
			func: $fun:expr,
			$(
				{
					input: $input:expr,
					expected: $expected:expr $(,)?
				}
			),+ $(,)?
		) => {
			$(
				{
					let bytes = $fun($input);
					assert_eq!(&$input.to_be_bytes()[4 - bytes..], &$expected[..]);
				}
			)+
		}
	}

	#[test]
	fn integer_shrinking_unsigned() {
		int_test! {
			func: bytes_to_occupy_uint,
			{
				input: 0u32,
				expected: [0],
			},
			{
				input: 1u32,
				expected: [1],
			},
			{
				input: 32767u32,
				expected: [127, 255],
			},
			{
				input: 65535u32,
				expected: [255, 255],
			},
			{
				input: 8_388_607_u32,
				expected: [0, 127, 255, 255],
			},
			{
				input: 16_777_215_u32,
				expected: [0, 255, 255, 255],
			},
			{
				input: u32::MAX,
				expected: [255, 255, 255, 255],
			},
		}
	}
}
