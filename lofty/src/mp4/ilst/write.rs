use super::data_type::DataType;
use super::r#ref::IlstRef;
use crate::config::{ParseOptions, WriteOptions};
use crate::error::{FileEncodingError, LoftyError, Result};
use crate::file::FileType;
use crate::macros::{decode_err, err, try_vec};
use crate::mp4::AtomData;
use crate::mp4::atom_info::{ATOM_HEADER_LEN, AtomIdent, AtomInfo, FOURCC_LEN};
use crate::mp4::ilst::r#ref::AtomRef;
use crate::mp4::read::{AtomReader, atom_tree, find_child_atom, meta_is_full, verify_mp4};
use crate::mp4::write::{AtomWriter, AtomWriterCompanion, ContextualAtom};
use crate::picture::{MimeType, Picture};
use crate::util::alloc::VecFallibleCapacity;
use crate::util::io::{FileLike, Length, Truncate};

use std::io::{Cursor, Seek, SeekFrom, Write};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

// A "full" atom is a traditional length + identifier, followed by a version (1) and flags (3)
const FULL_ATOM_SIZE: u64 = ATOM_HEADER_LEN + 4;
const HDLR_SIZE: u64 = ATOM_HEADER_LEN + 25;

// TODO: We are forcing the use of ParseOptions::DEFAULT_PARSING_MODE. This is not good. It should be caller-specified.
pub(crate) fn write_to<'a, F, I>(
	file: &mut F,
	tag: &mut IlstRef<'a, I>,
	write_options: WriteOptions,
) -> Result<()>
where
	F: FileLike,
	LoftyError: From<<F as Truncate>::Error>,
	LoftyError: From<<F as Length>::Error>,
	I: IntoIterator<Item = &'a AtomData> + 'a,
{
	log::debug!("Attempting to write `ilst` tag to file");

	// Create a temporary `AtomReader`, just to verify that this is a valid MP4 file
	let mut reader = AtomReader::new(file, ParseOptions::DEFAULT_PARSING_MODE)?;
	verify_mp4(&mut reader)?;

	// Now we can just read the entire file into memory
	let file = reader.into_inner();
	file.rewind()?;

	let mut atom_writer = AtomWriter::new_from_file(file, ParseOptions::DEFAULT_PARSING_MODE)?;

	let Some(moov) = atom_writer.find_contextual_atom(*b"moov") else {
		return Err(FileEncodingError::new(
			FileType::Mp4,
			"Could not find \"moov\" atom in target file",
		)
		.into());
	};

	let moov_start = moov.info.start;
	let moov_len = moov.info.len;
	let moov_extended = moov.info.extended;

	log::trace!(
		"Found `moov` atom, offset: {}, size: {}",
		moov_start,
		moov_len
	);

	let mut moov_data_start = moov_start + ATOM_HEADER_LEN;
	if moov_extended {
		moov_data_start += 8;
	}

	let mut write_handle = atom_writer.start_write();
	write_handle.seek(SeekFrom::Start(moov_data_start))?;

	let ilst = build_ilst(&mut tag.atoms)?;
	let remove_tag = ilst.is_empty();

	let udta = find_child_atom(
		&mut write_handle,
		moov_len,
		*b"udta",
		ParseOptions::DEFAULT_PARSING_MODE,
	)?;

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
		log::trace!(
			"Found `udta` atom, offset: {}, size: {}",
			udta.start,
			udta.len
		);

		existing_udta_size = udta.len;
		new_udta_size = existing_udta_size;

		let meta = find_child_atom(
			&mut write_handle,
			udta.len,
			*b"meta",
			ParseOptions::DEFAULT_PARSING_MODE,
		)?;

		// Nothing to do
		if remove_tag && meta.is_none() {
			return Ok(());
		}

		match meta {
			Some(meta) => {
				log::trace!(
					"Found `meta` atom, offset: {}, size: {}",
					meta.start,
					meta.len
				);

				// We may encounter a non-full `meta` atom
				meta_is_full(&mut write_handle)?;
				drop(write_handle);

				// We can use the existing `udta` and `meta` atoms
				save_to_existing(
					&atom_writer,
					moov,
					(meta, udta),
					&mut new_udta_size,
					ilst,
					remove_tag,
					write_options,
				)?
			},
			// We have to create the `meta` atom
			None => {
				log::trace!("No `meta` atom found, creating one");

				drop(write_handle);

				existing_udta_size = udta.len;

				// `meta` + `ilst`
				let capacity = FULL_ATOM_SIZE as usize + ilst.len();
				let buf = Vec::with_capacity(capacity);

				let bytes;
				{
					let meta_writer = AtomWriter::new(buf, ParseOptions::DEFAULT_PARSING_MODE);
					create_meta(&meta_writer, &ilst)?;

					bytes = meta_writer.into_contents();
				}

				write_handle = atom_writer.start_write();

				new_udta_size = udta.len + bytes.len() as u64;

				write_handle.seek(SeekFrom::Start(udta.start))?;
				write_handle.write_atom_size(udta.start, new_udta_size, udta.extended)?;

				// We'll put the new `meta` atom right at the start of `udta`
				let meta_start_pos = (udta.start + ATOM_HEADER_LEN) as usize;
				write_handle.splice(meta_start_pos..meta_start_pos, bytes);

				// TODO: We need to drop the handle at the end of each branch, which is annoying
				//       This whole function needs to be refactored eventually.
				drop(write_handle);
			},
		}
	} else {
		log::trace!("No `udta` atom found, creating one");

		// We have to create the `udta` atom
		let bytes = create_udta(&ilst)?;
		new_udta_size = bytes.len() as u64;

		// We'll put the new `udta` atom right at the start of `moov`
		let udta_pos = (moov_start + ATOM_HEADER_LEN) as usize;
		write_handle.splice(udta_pos..udta_pos, bytes);

		drop(write_handle);
	}

	let mut write_handle = atom_writer.start_write();

	write_handle.seek(SeekFrom::Start(moov_start))?;

	// Change the size of the moov atom
	let new_moov_length = (moov_len - existing_udta_size) + new_udta_size;

	log::trace!(
		"Updating `moov` atom size, old size: {}, new size: {}",
		moov_len,
		new_moov_length
	);
	write_handle.write_atom_size(moov_start, new_moov_length, moov_extended)?;

	drop(write_handle);

	atom_writer.save_to(file)?;

	Ok(())
}

// TODO: We are forcing the use of ParseOptions::DEFAULT_PARSING_MODE. This is not good. It should be caller-specified.
fn save_to_existing(
	writer: &AtomWriter,
	moov: &ContextualAtom,
	(meta, udta): (AtomInfo, AtomInfo),
	new_udta_size: &mut u64,
	ilst: Vec<u8>,
	remove_tag: bool,
	write_options: WriteOptions,
) -> Result<()> {
	let mut replacement;
	let range;

	let mut write_handle = writer.start_write();

	let (ilst_idx, tree) = atom_tree(
		&mut write_handle,
		meta.len - ATOM_HEADER_LEN,
		b"ilst",
		ParseOptions::DEFAULT_PARSING_MODE,
	)?;

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
				let mut i = ilst_idx;
				while i != 0 {
					let atom = &tree[i - 1];
					if atom.ident != AtomIdent::Fourcc(*b"free") {
						break;
					}

					available_space += atom.len;
					range_start = atom.start;
					i -= 1;
				}

				log::trace!("Found {} preceding `free` atoms", ilst_idx - i)
			}

			// And after
			if ilst_idx != tree.len() - 1 {
				let mut i = ilst_idx;
				while i < tree.len() - 1 {
					let atom = &tree[i + 1];
					if atom.ident != AtomIdent::Fourcc(*b"free") {
						break;
					}

					available_space += atom.len;
					i += 1;
				}

				log::trace!("Found {} succeeding `free` atoms", i - ilst_idx)
			}

			let ilst_len = ilst.len() as u64;

			// Check if we have enough padding to fit the `ilst` atom and a new `free` atom
			if available_space > ilst_len && (available_space - ilst_len) > 8 {
				// We have enough space to make use of the padding
				log::trace!("Found enough padding to fit the tag, file size will not change");

				let remaining_space = available_space - ilst_len;
				if remaining_space > u64::from(u32::MAX) {
					err!(TooMuchData);
				}

				let remaining_space = remaining_space as u32;

				write_handle.seek(SeekFrom::Start(range_start))?;
				write_handle.write_all(&ilst)?;

				// Write the remaining padding
				write_free_atom(&mut write_handle, remaining_space)?;

				return Ok(());
			}

			replacement = ilst;
			range = range_start as usize..range_end as usize;
		}
	}

	drop(write_handle);

	let mut new_meta_size = (meta.len - range.len() as u64) + replacement.len() as u64;

	// Pad the `ilst` in the event of a shrink
	let mut difference = (new_meta_size as i64) - (meta.len as i64);
	if !replacement.is_empty() && difference != 0 {
		log::trace!("Tag size changed, attempting to avoid offset update");

		let mut ilst_writer = Cursor::new(replacement);
		let (atom_size_difference, padding_size) =
			pad_atom(&mut ilst_writer, difference, write_options)?;

		replacement = ilst_writer.into_inner();
		new_meta_size += padding_size;
		difference = atom_size_difference;
	}

	// Update the parent atom sizes
	if new_meta_size != meta.len {
		// We need to change the `meta` and `udta` atom sizes
		let mut write_handle = writer.start_write();

		*new_udta_size = (udta.len - meta.len) + new_meta_size;

		write_handle.seek(SeekFrom::Start(meta.start))?;
		write_handle.write_atom_size(meta.start, new_meta_size, meta.extended)?;

		write_handle.seek(SeekFrom::Start(udta.start))?;
		write_handle.write_atom_size(udta.start, *new_udta_size, udta.extended)?;
		drop(write_handle);
	}

	// Update offset atoms
	if difference != 0 {
		let offset = range.start as u64;
		update_offsets(writer, moov, difference, offset)?;
	}

	// Replace the `ilst` atom
	let mut write_handle = writer.start_write();
	write_handle.splice(range, replacement);
	drop(write_handle);

	Ok(())
}

fn pad_atom<W>(
	writer: &mut W,
	mut atom_size_difference: i64,
	write_options: WriteOptions,
) -> Result<(i64, u64)>
where
	W: Write + Seek,
{
	if atom_size_difference.is_positive() {
		log::trace!("Atom has grown, cannot avoid offset update");
		return Ok((atom_size_difference, 0));
	}

	// When the tag shrinks, we need to try and pad it out to avoid updating
	// the offsets.
	writer.seek(SeekFrom::End(0))?;

	let padding_size: u64;
	let diff_abs = atom_size_difference.abs();
	if diff_abs >= ATOM_HEADER_LEN as i64 {
		log::trace!(
			"Avoiding offset update, padding atom with {} bytes",
			diff_abs
		);

		// If our difference is >= 8, we can make up the difference with
		// a `free` atom and skip updating the offsets.
		write_free_atom(writer, diff_abs as u32)?;
		atom_size_difference = 0;
		padding_size = diff_abs as u64;

		return Ok((atom_size_difference, padding_size));
	}

	let Some(preferred_padding) = write_options.preferred_padding else {
		log::trace!("Cannot avoid offset update, not padding atom");
		return Ok((atom_size_difference, 0));
	};

	log::trace!(
		"Cannot avoid offset update, padding atom with {} bytes",
		preferred_padding
	);

	// Otherwise, we'll have to just pad the default amount,
	// and update the offsets.
	write_free_atom(writer, preferred_padding)?;
	atom_size_difference += i64::from(preferred_padding);
	padding_size = u64::from(preferred_padding);

	Ok((atom_size_difference, padding_size))
}

fn write_free_atom<W>(writer: &mut W, size: u32) -> Result<()>
where
	W: Write,
{
	writer.write_u32::<BigEndian>(size)?;
	writer.write_all(b"free")?;
	writer.write_all(&try_vec![1; (size - ATOM_HEADER_LEN as u32) as usize])?;
	Ok(())
}

fn update_offsets(
	writer: &AtomWriter,
	moov: &ContextualAtom,
	difference: i64,
	ilst_offset: u64,
) -> Result<()> {
	log::debug!("Checking for offset atoms to update");

	let mut write_handle = writer.start_write();

	// 32-bit offsets
	for stco in moov.find_all_children(*b"stco", true) {
		log::trace!("Found `stco` atom");

		let stco_start = stco.start;
		if stco.extended {
			decode_err!(@BAIL Mp4, "Found an extended `stco` atom");
		}

		write_handle.seek(SeekFrom::Start(stco_start + ATOM_HEADER_LEN + 4))?;

		let count = write_handle.read_u32::<BigEndian>()?;
		for _ in 0..count {
			let read_offset = write_handle.read_u32::<BigEndian>()?;
			if u64::from(read_offset) < ilst_offset {
				continue;
			}
			write_handle.seek(SeekFrom::Current(-4))?;
			write_handle.write_u32::<BigEndian>((i64::from(read_offset) + difference) as u32)?;

			log::trace!(
				"Updated offset from {} to {}",
				read_offset,
				(i64::from(read_offset) + difference) as u32
			);
		}
	}

	// 64-bit offsets
	for co64 in moov.find_all_children(*b"co64", true) {
		log::trace!("Found `co64` atom");

		let co64_start = co64.start;
		if !co64.extended {
			decode_err!(@BAIL Mp4, "Expected `co64` atom to be extended");
		}

		write_handle.seek(SeekFrom::Start(co64_start + ATOM_HEADER_LEN + 8 + 4))?;

		let count = write_handle.read_u32::<BigEndian>()?;
		for _ in 0..count {
			let read_offset = write_handle.read_u64::<BigEndian>()?;
			if read_offset < ilst_offset {
				continue;
			}

			write_handle.seek(SeekFrom::Current(-8))?;
			write_handle.write_u64::<BigEndian>((read_offset as i64 + difference) as u64)?;

			log::trace!(
				"Updated offset from {} to {}",
				read_offset,
				((read_offset as i64) + difference) as u64
			);
		}
	}

	let Some(moof) = writer.find_contextual_atom(*b"moof") else {
		return Ok(());
	};

	log::trace!("Found `moof` atom, checking for `tfhd` atoms to update");

	// 64-bit offsets
	for tfhd in moof.find_all_children(*b"tfhd", true) {
		log::trace!("Found `tfhd` atom");

		let tfhd_start = tfhd.start;
		if tfhd.extended {
			decode_err!(@BAIL Mp4, "Found an extended `tfhd` atom");
		}

		// Skip atom header + version (1)
		write_handle.seek(SeekFrom::Start(tfhd_start + ATOM_HEADER_LEN + 1))?;

		let flags = write_handle.read_u24::<BigEndian>()?;
		let base_data_offset = (flags & 0b1) != 0;

		if base_data_offset {
			let read_offset = write_handle.read_u64::<BigEndian>()?;
			if read_offset < ilst_offset {
				continue;
			}

			write_handle.seek(SeekFrom::Current(-8))?;
			write_handle.write_u64::<BigEndian>((read_offset as i64 + difference) as u64)?;

			log::trace!(
				"Updated offset from {} to {}",
				read_offset,
				((read_offset as i64) + difference) as u64
			);
		}
	}

	drop(write_handle);

	Ok(())
}

fn create_udta(ilst: &[u8]) -> Result<Vec<u8>> {
	const UDTA_HEADER: [u8; 8] = [0, 0, 0, 0, b'u', b'd', b't', b'a'];

	// `udta` + `meta` + `hdlr` + `ilst`
	let capacity = ATOM_HEADER_LEN + FULL_ATOM_SIZE + HDLR_SIZE + ilst.len() as u64;
	let mut buf = Vec::try_with_capacity_stable(capacity as usize)?;

	buf.write_all(&UDTA_HEADER)?;

	let udta_writer = AtomWriter::new(buf, ParseOptions::DEFAULT_PARSING_MODE);
	let mut write_handle = udta_writer.start_write();

	write_handle.seek(SeekFrom::Current(UDTA_HEADER.len() as i64))?; // Skip header
	drop(write_handle);

	create_meta(&udta_writer, ilst)?;

	// `udta` size
	{
		let mut write_handle = udta_writer.start_write();
		write_handle.rewind()?;
		write_handle.write_atom_size(0, write_handle.len() as u64, false)?;
	}

	Ok(udta_writer.into_contents())
}

fn create_meta(writer: &AtomWriter, ilst: &[u8]) -> Result<()> {
	let mut write_handle = writer.start_write();

	let start = write_handle.stream_position()?;
	// meta atom
	write_handle.write_all(&[0, 0, 0, 0, b'm', b'e', b't', b'a', 0, 0, 0, 0])?;

	// hdlr atom
	write_handle.write_u32::<BigEndian>(0)?;
	write_handle.write_all(b"hdlr")?;
	write_handle.write_u64::<BigEndian>(0)?;
	write_handle.write_all(b"mdirappl")?;
	write_handle.write_all(&[0, 0, 0, 0, 0, 0, 0, 0, 0])?;

	write_handle.seek(SeekFrom::Start(start))?;

	let meta_size = FULL_ATOM_SIZE + HDLR_SIZE + ilst.len() as u64;
	write_handle.write_atom_size(start, meta_size, false)?;

	// Seek to `hdlr` size
	let hdlr_size_pos = write_handle.seek(SeekFrom::Current(4))?;
	write_handle.write_atom_size(hdlr_size_pos, HDLR_SIZE, false)?;

	write_handle.seek(SeekFrom::End(0))?;
	write_handle.write_all(ilst)?;

	Ok(())
}

pub(super) fn build_ilst<'a, I>(atoms: &mut dyn Iterator<Item = AtomRef<'a, I>>) -> Result<Vec<u8>>
where
	I: IntoIterator<Item = &'a AtomData> + 'a,
{
	log::debug!("Building `ilst` atom");

	let mut peek = atoms.peekable();

	if peek.peek().is_none() {
		return Ok(Vec::new());
	}

	let ilst_header = vec![0, 0, 0, 0, b'i', b'l', b's', b't'];
	let ilst_writer = AtomWriter::new(ilst_header, ParseOptions::DEFAULT_PARSING_MODE);

	let mut write_handle = ilst_writer.start_write();
	write_handle.seek(SeekFrom::End(0))?;

	for atom in peek {
		let start = write_handle.stream_position()?;

		// Empty size, we get it later
		write_handle.write_all(&[0; FOURCC_LEN as usize])?;

		match atom.ident {
			AtomIdent::Fourcc(ref fourcc) => write_handle.write_all(fourcc)?,
			AtomIdent::Freeform { mean, name } => write_freeform(&mean, &name, &mut write_handle)?,
		}

		write_atom_data(atom.data, &mut write_handle)?;

		let end = write_handle.stream_position()?;

		let size = end - start;

		write_handle.seek(SeekFrom::Start(start))?;

		write_handle.write_atom_size(start, size, false)?;

		write_handle.seek(SeekFrom::Start(end))?;
	}

	let size = write_handle.len();

	write_handle.rewind()?;

	write_handle.write_atom_size(0, size as u64, false)?;

	drop(write_handle);

	log::trace!("Built `ilst` atom, size: {} bytes", size);

	Ok(ilst_writer.into_contents())
}

fn write_freeform<W>(mean: &str, name: &str, writer: &mut W) -> Result<()>
where
	W: Write,
{
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

fn write_atom_data<'a, I>(data: I, writer: &mut AtomWriterCompanion<'_>) -> Result<()>
where
	I: IntoIterator<Item = &'a AtomData> + 'a,
{
	for value in data {
		match value {
			AtomData::UTF8(text) => write_data(DataType::Utf8, text.as_bytes(), writer)?,
			AtomData::UTF16(text) => write_data(DataType::Utf16, text.as_bytes(), writer)?,
			AtomData::Picture(pic) => write_picture(pic, writer)?,
			AtomData::SignedInteger(int) => write_signed_int(*int, writer)?,
			AtomData::UnsignedInteger(uint) => write_unsigned_int(*uint, writer)?,
			AtomData::Bool(b) => write_bool(*b, writer)?,
			AtomData::Unknown { code, data } => write_data(*code, data, writer)?,
		}
	}

	Ok(())
}

fn write_signed_int(int: i32, writer: &mut AtomWriterCompanion<'_>) -> Result<()> {
	write_int(DataType::BeSignedInteger, int.to_be_bytes(), 4, writer)
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

fn write_unsigned_int(uint: u32, writer: &mut AtomWriterCompanion<'_>) -> Result<()> {
	let bytes_needed = bytes_to_occupy_uint(uint);
	write_int(
		DataType::BeUnsignedInteger,
		uint.to_be_bytes(),
		bytes_needed,
		writer,
	)
}

fn write_int(
	flags: DataType,
	bytes: [u8; 4],
	bytes_needed: usize,
	writer: &mut AtomWriterCompanion<'_>,
) -> Result<()> {
	debug_assert!(bytes_needed != 0);
	write_data(flags, &bytes[4 - bytes_needed..], writer)
}

fn write_bool(b: bool, writer: &mut AtomWriterCompanion<'_>) -> Result<()> {
	write_int(
		DataType::BeSignedInteger,
		i32::from(b).to_be_bytes(),
		1,
		writer,
	)
}

fn write_picture(picture: &Picture, writer: &mut AtomWriterCompanion<'_>) -> Result<()> {
	match picture.mime_type {
		// GIF is deprecated
		Some(MimeType::Gif) => write_data(DataType::Gif, &picture.data, writer),
		Some(MimeType::Jpeg) => write_data(DataType::Jpeg, &picture.data, writer),
		Some(MimeType::Png) => write_data(DataType::Png, &picture.data, writer),
		Some(MimeType::Bmp) => write_data(DataType::Bmp, &picture.data, writer),
		// We'll assume implicit (0) was the intended type
		None => write_data(DataType::Reserved, &picture.data, writer),
		_ => Err(FileEncodingError::new(
			FileType::Mp4,
			"Attempted to write an unsupported picture format",
		)
		.into()),
	}
}

fn write_data(flags: DataType, data: &[u8], writer: &mut AtomWriterCompanion<'_>) -> Result<()> {
	if u32::from(flags) > DataType::MAX {
		return Err(FileEncodingError::new(
			FileType::Mp4,
			"Attempted to write a code that cannot fit in 24 bits",
		)
		.into());
	}

	// .... DATA (version = 0) (flags) (locale = 0000) (data)
	let size = FULL_ATOM_SIZE + 4 + data.len() as u64;

	writer.write_all(&[0, 0, 0, 0, b'd', b'a', b't', b'a'])?;
	let start = writer.seek(SeekFrom::Current(-8))?;
	writer.write_atom_size(start, size, false)?;

	// Version
	writer.write_u8(0)?;

	writer.write_u24::<BigEndian>(u32::from(flags))?;

	// Locale
	writer.write_all(&[0; 4])?;
	writer.write_all(data)?;

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::bytes_to_occupy_uint;

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

	#[test_log::test]
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
