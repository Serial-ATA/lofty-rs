mod atom_reader;

use super::Mp4File;
use super::atom_info::{AtomIdent, AtomInfo};
use super::moov::Moov;
use super::properties::Mp4Properties;
use crate::config::{ParseOptions, ParsingMode};
use crate::error::{ErrorKind, LoftyError, Result};
use crate::macros::{decode_err, err};
use crate::util::io::SeekStreamLen;
use crate::util::text::utf8_decode_str;

use std::io::{Read, Seek, SeekFrom};

use byteorder::{BigEndian, ReadBytesExt};

pub(super) use atom_reader::AtomReader;

pub(in crate::mp4) fn verify_mp4<R>(reader: &mut AtomReader<R>) -> Result<String>
where
	R: Read + Seek,
{
	let Some(atom) = reader.next()? else {
		err!(UnknownFormat);
	};

	if atom.ident != AtomIdent::Fourcc(*b"ftyp") {
		err!(UnknownFormat);
	}

	// size + identifier + major brand
	// There *should* be more, but this is all we need from it
	if atom.len < 12 {
		decode_err!(@BAIL Mp4, "\"ftyp\" atom too short");
	}

	let mut major_brand = [0u8; 4];
	reader.read_exact(&mut major_brand)?;

	reader.seek(SeekFrom::Current((atom.len - 12) as i64))?;

	let major_brand = utf8_decode_str(&major_brand)
		.map(ToOwned::to_owned)
		.map_err(|_| {
			LoftyError::new(ErrorKind::BadAtom("Unable to parse \"ftyp\"'s major brand"))
		})?;

	log::debug!("Verified to be an MP4 file. Major brand: {}", major_brand);
	Ok(major_brand)
}

#[allow(unstable_name_collisions)]
pub(crate) fn read_from<R>(data: &mut R, parse_options: ParseOptions) -> Result<Mp4File>
where
	R: Read + Seek,
{
	let mut reader = AtomReader::new(data, parse_options.parsing_mode)?;
	let file_length = reader.stream_len_hack()?;

	let ftyp = verify_mp4(&mut reader)?;

	// Find the `moov` atom and restrict the reader to its length
	let moov_info = Moov::find(&mut reader)?;
	reader.reset_bounds(moov_info.start + 8, moov_info.len - 8);

	let moov = Moov::parse(&mut reader, parse_options)?;

	Ok(Mp4File {
		ftyp,
		ilst_tag: moov.ilst,
		properties: if parse_options.read_properties {
			// Remove the length restriction
			reader.reset_bounds(0, file_length);
			super::properties::read_properties(
				&mut reader,
				&moov.traks,
				file_length,
				parse_options.parsing_mode,
			)?
		} else {
			Mp4Properties::default()
		},
	})
}

/// Seeks the reader to the end of the atom
///
/// This should be used immediately after [`AtomInfo::read`] to skip an unwanted atom.
///
/// NOTES:
///
/// * This makes the assumption that the reader is at the end of the atom's header.
/// * This makes the assumption that the `len` is the *full atom length*, not just that of the content.
pub(super) fn skip_atom<R>(reader: &mut R, extended: bool, len: u64) -> Result<()>
where
	R: Read + Seek,
{
	log::trace!("Attempting to skip {} bytes", len - 8);

	if !extended {
		reader.seek(SeekFrom::Current(i64::from(len as u32) - 8))?;
		return Ok(());
	}

	let pos = reader.stream_position()?;

	if let (pos, false) = pos.overflowing_add(len - 8) {
		reader.seek(SeekFrom::Start(pos))?;
	} else {
		err!(TooMuchData);
	}

	Ok(())
}

/// Finds the first child atom with the given fourcc
///
/// * `len` is the length of the parent atom
/// * `expected` is the fourcc of the child atom to find
pub(super) fn find_child_atom<R>(
	reader: &mut R,
	mut len: u64,
	expected: [u8; 4],
	parse_mode: ParsingMode,
) -> Result<Option<AtomInfo>>
where
	R: Read + Seek,
{
	let mut ret = None;

	while len > 8 {
		let Some(atom) = AtomInfo::read(reader, len, parse_mode)? else {
			break;
		};

		match atom.ident {
			AtomIdent::Fourcc(fourcc) if fourcc == expected => {
				ret = Some(atom);
				break;
			},
			_ => {
				skip_atom(reader, atom.extended, atom.len)?;
				len = len.saturating_sub(atom.len);
			},
		}
	}

	Ok(ret)
}

// Creates a tree of nested atoms
pub(super) fn atom_tree<R>(
	reader: &mut R,
	mut len: u64,
	up_to: &[u8],
	parse_mode: ParsingMode,
) -> Result<(usize, Vec<AtomInfo>)>
where
	R: Read + Seek,
{
	let mut found_idx: usize = 0;
	let mut buf = Vec::new();

	let mut i = 0;

	while len > 8 {
		let Some(atom) = AtomInfo::read(reader, len, parse_mode)? else {
			break;
		};

		skip_atom(reader, atom.extended, atom.len)?;
		len = len.saturating_sub(atom.len);

		if let AtomIdent::Fourcc(ref fourcc) = atom.ident {
			i += 1;

			if fourcc == up_to {
				found_idx = i;
			}

			buf.push(atom);
		}
	}

	found_idx = found_idx.saturating_sub(1);

	Ok((found_idx, buf))
}

pub(super) fn meta_is_full<R>(reader: &mut R) -> Result<bool>
where
	R: Read + Seek,
{
	// A full `meta` atom should have the following:
	//
	// Version (1)
	// Flags (3)
	//
	// However, it's possible that it is written as a normal atom,
	// meaning this would be the size of the next atom.
	let _version_flags = reader.read_u32::<BigEndian>()?;

	// Check if the next four bytes is one of the nested `meta` atoms
	let mut possible_ident = [0; 4];
	reader.read_exact(&mut possible_ident)?;

	match &possible_ident {
		b"hdlr" | b"ilst" | b"mhdr" | b"ctry" | b"lang" => {
			log::warn!("File contains a non-full 'meta' atom");

			reader.seek(SeekFrom::Current(-8))?;
			Ok(false)
		},
		_ => {
			reader.seek(SeekFrom::Current(-4))?;
			Ok(true)
		},
	}
}
