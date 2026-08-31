use super::data_type::DataType;
use super::r#ref::IlstRef;
use crate::config::WriteOptions;
use crate::error::{FileEncodingError, FileParseError, TagEncodingError, TooMuchDataError};
use crate::file::FileType;
use crate::io::VerifiedFile;
use crate::macros::try_vec;
use crate::mp4::AtomData;
use crate::mp4::atom_info::{ATOM_HEADER_LEN, AtomIdent, AtomInfo, FOURCC_LEN};
use crate::mp4::error::{AtomParseError, Mp4ParseError};
use crate::mp4::ilst::error::IlstEncodingError;
use crate::mp4::ilst::r#ref::AtomRef;
use crate::mp4::read::{AtomReader, atom_tree, find_child_atom, meta_is_full, verify_mp4};
use crate::mp4::write::{AtomWriter, AtomWriterCompanion};
use crate::picture::{MimeType, Picture};
use crate::util::alloc::VecFallibleCapacity;
use crate::util::io::FileLike;

use std::io::{Cursor, Seek, SeekFrom, Write};

use byteorder::{BigEndian, WriteBytesExt};

// A "full" atom is a traditional length + identifier, followed by a version (1) and flags (3)
const FULL_ATOM_SIZE: u64 = ATOM_HEADER_LEN + 4;

fn handle_atom_parse_error(error: AtomParseError) -> FileEncodingError {
	FileEncodingError::new(FileType::Mp4, error.into())
}

pub(crate) fn write_to<'a, F, I>(
	file: VerifiedFile<'_, F>,
	tag: &mut IlstRef<'a, I>,
	write_options: WriteOptions,
) -> Result<(), FileEncodingError>
where
	F: FileLike,
	I: IntoIterator<Item = &'a AtomData> + 'a,
{
	log::debug!("Attempting to write `ilst` tag to file");

	// Create a temporary `AtomReader`, just to verify that this is a valid MP4 file
	let file = file.into_inner();
	let mut reader = AtomReader::new(file, write_options.parse_options.parsing_mode)?;
	verify_mp4(&mut reader).map_err(FileParseError::from)?;

	// Now we can just read the entire file into memory
	let mut file = reader.into_inner();
	file.rewind()?;

	let mut atom_writer =
		AtomWriter::new_from_file(&mut file, write_options.parse_options.parsing_mode)
			.map_err(Into::<FileParseError>::into)?;

	let Some(moov) = atom_writer.atoms().find_atom(*b"moov") else {
		return Err(FileParseError::from(Mp4ParseError::missing_moov()).into());
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

	let ilst = build_ilst(&mut tag.atoms, write_options).map_err(TagEncodingError::from)?;
	let remove_tag = ilst.is_empty();

	let udta = find_child_atom(
		&mut write_handle,
		moov_len,
		*b"udta",
		write_options.parse_options.parsing_mode,
	)
	.map_err(handle_atom_parse_error)?;

	// Nothing to do
	if remove_tag && udta.is_none() {
		write_handle.finish()?;
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
			write_options.parse_options.parsing_mode,
		)
		.map_err(handle_atom_parse_error)?;

		// Nothing to do
		if remove_tag && meta.is_none() {
			write_handle.finish()?;
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
				meta_is_full(&mut write_handle).map_err(handle_atom_parse_error)?;

				// We can use the existing `udta` and `meta` atoms
				save_to_existing(
					&mut write_handle,
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

				existing_udta_size = udta.len;

				// We'll put the new `meta` atom right at the start of `udta`
				let meta_start_pos = udta.start + ATOM_HEADER_LEN;
				write_handle.seek(SeekFrom::Start(meta_start_pos))?;
				let meta_size = create_meta(&mut write_handle, &ilst)?;

				new_udta_size = udta.len + meta_size;
				write_handle.seek(SeekFrom::Start(udta.start))?;
				write_handle.write_atom_size(udta.start, new_udta_size, udta.extended)?;
			},
		}
	} else {
		log::trace!("No `udta` atom found, creating one");

		// We have to create the `udta` atom. Put it right at the start of `moov`.
		let udta_pos = moov_start + ATOM_HEADER_LEN;
		write_handle.seek(SeekFrom::Start(udta_pos))?;
		new_udta_size = create_udta(&mut write_handle, &ilst, write_options)?;
	}

	write_handle.seek(SeekFrom::Start(moov_start))?;

	// Change the size of the moov atom
	let new_moov_length = (moov_len - existing_udta_size) + new_udta_size;

	log::trace!(
		"Updating `moov` atom size, old size: {}, new size: {}",
		moov_len,
		new_moov_length
	);
	write_handle.write_atom_size(moov_start, new_moov_length, moov_extended)?;

	write_handle.finish()?;

	atom_writer.save_to(&mut file)?;

	Ok(())
}

fn save_to_existing(
	writer: &mut AtomWriterCompanion<'_>,
	(meta, udta): (AtomInfo, AtomInfo),
	new_udta_size: &mut u64,
	ilst: Vec<u8>,
	remove_tag: bool,
	write_options: WriteOptions,
) -> Result<(), FileEncodingError> {
	let mut replacement;
	let range;

	let (ilst_idx, tree) = atom_tree(
		writer,
		meta.len - ATOM_HEADER_LEN,
		b"ilst",
		write_options.parse_options.parsing_mode,
	)
	.map_err(Into::<FileParseError>::into)?;

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
					return Err(TooMuchDataError.into());
				}

				let remaining_space = remaining_space as u32;

				writer.seek(SeekFrom::Start(range_start))?;
				writer.write_all(&ilst)?;

				// Write the remaining padding
				write_free_atom(writer, remaining_space)?;

				return Ok(());
			}

			replacement = ilst;
			range = range_start as usize..range_end as usize;
		}
	}

	let mut new_meta_size = (meta.len - range.len() as u64) + replacement.len() as u64;

	let difference = (new_meta_size as i64) - (meta.len as i64);
	if !replacement.is_empty() && difference != 0 {
		log::trace!("Tag size changed, attempting to avoid offset update");

		let mut ilst_writer = Cursor::new(replacement);
		let (_, padding_size) = pad_atom(&mut ilst_writer, difference, write_options)?;

		replacement = ilst_writer.into_inner();
		new_meta_size += padding_size;
	}

	// Update the parent atom sizes
	if new_meta_size != meta.len {
		// We need to change the `meta` and `udta` atom sizes
		*new_udta_size = (udta.len - meta.len) + new_meta_size;

		writer.seek(SeekFrom::Start(meta.start))?;
		writer.write_atom_size(meta.start, new_meta_size, meta.extended)?;

		writer.seek(SeekFrom::Start(udta.start))?;
		writer.write_atom_size(udta.start, *new_udta_size, udta.extended)?;
	}

	// Replace the `ilst` atom
	writer.splice(range, replacement);

	Ok(())
}

fn pad_atom<W>(
	mut writer: W,
	mut atom_size_difference: i64,
	write_options: WriteOptions,
) -> Result<(i64, u64), FileEncodingError>
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
		write_free_atom(&mut writer, diff_abs as u32)?;
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
	write_free_atom(&mut writer, preferred_padding.get())?;
	atom_size_difference += i64::from(preferred_padding.get());
	padding_size = u64::from(preferred_padding.get());

	Ok((atom_size_difference, padding_size))
}

fn write_free_atom<W>(writer: &mut W, size: u32) -> Result<(), FileEncodingError>
where
	W: Write,
{
	writer.write_u32::<BigEndian>(size)?;
	writer.write_all(b"free")?;
	writer.write_all(&try_vec![1; (size - ATOM_HEADER_LEN as u32) as usize]?)?;
	Ok(())
}

/// Write a `moov.udta` atom at the current position
///
/// This returns the size of the `udta` atom in bytes.
fn create_udta(
	writer: &mut AtomWriterCompanion<'_>,
	ilst: &[u8],
	write_options: WriteOptions,
) -> Result<u64, FileEncodingError> {
	const UDTA_HEADER: [u8; 8] = [0, 0, 0, 0, b'u', b'd', b't', b'a'];

	// `udta` + `meta` + `hdlr` + `ilst`
	let capacity =
		((ATOM_HEADER_LEN as usize) + META_ATOM.len() + HDLR_ATOM.len() + ilst.len()) as u64;
	let mut buf = Vec::try_with_capacity_stable(capacity as usize)?;

	buf.write_all(&UDTA_HEADER)?;

	let udta_writer = AtomWriter::new(buf, write_options.parse_options.parsing_mode);
	let mut write_handle = udta_writer.start_write();

	write_handle.seek(SeekFrom::Current(UDTA_HEADER.len() as i64))?; // Skip header

	create_meta(&mut write_handle, ilst)?;

	// `udta` size
	{
		write_handle.rewind()?;
		write_handle.write_atom_size(0, write_handle.len() as u64, false)?;
	}

	write_handle.finish()?;

	let udta = udta_writer.into_contents();
	let udta_size = udta.len() as u64;

	let pos: usize = writer
		.stream_position()?
		.try_into()
		.map_err(|_| TooMuchDataError)?;
	writer.splice(pos..pos, udta);

	Ok(udta_size)
}

// `moov.udta.meta`
const META_ATOM: [u8; 12] = [
	0, 0, 0, 0, // Size (written later)
	b'm', b'e', b't', b'a', // Name
	0, 0, 0, 0, // Version (1), Flags (3)
];

// `moov.udta.meta.hdlr`
const HDLR_ATOM: [u8; 33] = [
	0, 0, 0, 0, // Size (written later)
	b'h', b'd', b'l', b'r', // Name
	0, 0, 0, 0, // Version (1), Flags (3)
	0, 0, 0, 0, // Predefined (always 0)
	b'm', b'd', b'i', b'r', // Component subtype
	b'a', b'p', b'p', b'l', // Component manufacturer
	0, 0, 0, 0, 0, 0, 0, 0, 0, // Component flags/flags mask/name, all reserved
];

/// Write a `moov.udta.meta` atom at the current position
///
/// This returns the size of the `meta` atom in bytes.
#[rustfmt::skip]
fn create_meta(writer: &mut AtomWriterCompanion<'_>, ilst: &[u8]) -> Result<u64, FileEncodingError> {
	let start = writer.stream_position()?;
	let start_usize: usize = start.try_into().map_err(|_| TooMuchDataError)?;

	writer.splice(start_usize..start_usize, META_ATOM.into_iter().chain(HDLR_ATOM).chain(ilst.iter().copied()));

	writer.seek(SeekFrom::Start(start))?;

	let meta_size = FULL_ATOM_SIZE + HDLR_ATOM.len() as u64 + ilst.len() as u64;
	writer.write_atom_size(start, meta_size, false)?;

	// Seek to `hdlr` size
	let hdlr_size_pos = writer.seek(SeekFrom::Current(4))?;
	writer.write_atom_size(hdlr_size_pos, HDLR_ATOM.len() as u64, false)?;

	Ok((META_ATOM.len() + HDLR_ATOM.len() + ilst.len()) as u64)
}

pub(super) fn build_ilst<'a, I>(
	atoms: &mut dyn Iterator<Item = AtomRef<'a, I>>,
	write_options: WriteOptions,
) -> Result<Vec<u8>, IlstEncodingError>
where
	I: IntoIterator<Item = &'a AtomData> + 'a,
{
	log::debug!("Building `ilst` atom");

	let mut peek = atoms.peekable();

	if peek.peek().is_none() {
		return Ok(Vec::new());
	}

	let ilst_header = vec![0, 0, 0, 0, b'i', b'l', b's', b't'];
	let ilst_writer = AtomWriter::new(ilst_header, write_options.parse_options.parsing_mode);

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

	write_handle.finish()?;

	log::trace!("Built `ilst` atom, size: {size} bytes");

	Ok(ilst_writer.into_contents())
}

fn write_freeform<W>(mean: &str, name: &str, writer: &mut W) -> Result<(), IlstEncodingError>
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

fn write_atom_data<'a, I>(
	data: I,
	writer: &mut AtomWriterCompanion<'_>,
) -> Result<(), IlstEncodingError>
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

fn write_signed_int(
	int: i32,
	writer: &mut AtomWriterCompanion<'_>,
) -> Result<(), IlstEncodingError> {
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

fn write_unsigned_int(
	uint: u32,
	writer: &mut AtomWriterCompanion<'_>,
) -> Result<(), IlstEncodingError> {
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
) -> Result<(), IlstEncodingError> {
	debug_assert!(bytes_needed != 0);
	write_data(flags, &bytes[4 - bytes_needed..], writer)
}

fn write_bool(b: bool, writer: &mut AtomWriterCompanion<'_>) -> Result<(), IlstEncodingError> {
	write_int(
		DataType::BeSignedInteger,
		i32::from(b).to_be_bytes(),
		1,
		writer,
	)
}

fn write_picture(
	picture: &Picture,
	writer: &mut AtomWriterCompanion<'_>,
) -> Result<(), IlstEncodingError> {
	match picture.mime_type {
		// GIF is deprecated
		Some(MimeType::Gif) => write_data(DataType::Gif, &picture.data, writer),
		Some(MimeType::Jpeg) => write_data(DataType::Jpeg, &picture.data, writer),
		Some(MimeType::Png) => write_data(DataType::Png, &picture.data, writer),
		Some(MimeType::Bmp) => write_data(DataType::Bmp, &picture.data, writer),
		// We'll assume implicit (0) was the intended type
		None => write_data(DataType::Reserved, &picture.data, writer),
		_ => Err(IlstEncodingError::message(
			"attempted to write a picture with an unsupported MIME type",
		)),
	}
}

fn write_data(
	flags: DataType,
	data: &[u8],
	writer: &mut AtomWriterCompanion<'_>,
) -> Result<(), IlstEncodingError> {
	if u32::from(flags) > DataType::MAX {
		return Err(IlstEncodingError::message(
			"attempted to write a code that cannot fit in 24 bits",
		));
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
