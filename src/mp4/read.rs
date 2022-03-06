use super::atom_info::{AtomIdent, AtomInfo};
use super::moov::Moov;
use super::properties::Mp4Properties;
use super::Mp4File;
use crate::error::{ErrorKind, FileDecodingError, LoftyError, Result};
use crate::file::FileType;

use std::io::{Read, Seek, SeekFrom};

use byteorder::{BigEndian, ReadBytesExt};

pub(in crate::mp4) fn verify_mp4<R>(data: &mut R) -> Result<String>
where
	R: Read + Seek,
{
	let atom = AtomInfo::read(data)?;

	if atom.ident != AtomIdent::Fourcc(*b"ftyp") {
		return Err(LoftyError::new(ErrorKind::UnknownFormat));
	}

	// size + identifier + major brand
	// There *should* be more, but this is all we need from it
	if atom.len < 12 {
		return Err(FileDecodingError::new(FileType::MP4, "\"ftyp\" atom too short").into());
	}

	let mut major_brand = vec![0; 4];
	data.read_exact(&mut major_brand)?;

	data.seek(SeekFrom::Current((atom.len - 12) as i64))?;

	String::from_utf8(major_brand)
		.map_err(|_| LoftyError::new(ErrorKind::BadAtom("Unable to parse \"ftyp\"'s major brand")))
}

pub(crate) fn read_from<R>(data: &mut R, read_properties: bool) -> Result<Mp4File>
where
	R: Read + Seek,
{
	let ftyp = verify_mp4(data)?;

	Moov::find(data)?;
	let moov = Moov::parse(data, read_properties)?;

	let file_length = data.seek(SeekFrom::End(0))?;

	Ok(Mp4File {
		ftyp,
		#[cfg(feature = "mp4_ilst")]
		ilst: moov.meta,
		properties: if read_properties {
			super::properties::read_properties(data, &moov.traks, file_length)?
		} else {
			Mp4Properties::default()
		},
	})
}

pub(super) fn skip_unneeded<R>(data: &mut R, ext: bool, len: u64) -> Result<()>
where
	R: Read + Seek,
{
	if ext {
		let pos = data.seek(SeekFrom::Current(0))?;

		if let (pos, false) = pos.overflowing_add(len - 8) {
			data.seek(SeekFrom::Start(pos))?;
		} else {
			return Err(LoftyError::new(ErrorKind::TooMuchData));
		}
	} else {
		data.seek(SeekFrom::Current(i64::from(len as u32) - 8))?;
	}

	Ok(())
}

pub(super) fn nested_atom<R>(data: &mut R, len: u64, expected: &[u8]) -> Result<Option<AtomInfo>>
where
	R: Read + Seek,
{
	let mut read = 8;
	let mut ret = None;

	while read < len {
		let atom = AtomInfo::read(data)?;

		match atom.ident {
			AtomIdent::Fourcc(ref fourcc) if fourcc == expected => {
				ret = Some(atom);
				break;
			},
			_ => {
				skip_unneeded(data, atom.extended, atom.len)?;
				read += atom.len
			},
		}
	}

	Ok(ret)
}

// Creates a tree of nested atoms
pub(super) fn atom_tree<R>(data: &mut R, len: u64, up_to: &[u8]) -> Result<(usize, Vec<AtomInfo>)>
where
	R: Read + Seek,
{
	let mut read = 8;
	let mut found_idx: usize = 0;
	let mut buf = Vec::new();

	let mut i = 0;

	while read < len {
		let atom = AtomInfo::read(data)?;

		skip_unneeded(data, atom.extended, atom.len)?;
		read += atom.len;

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

pub(super) fn meta_is_full<R>(data: &mut R) -> Result<bool>
where
	R: Read + Seek,
{
	let meta_pos = data.stream_position()?;

	// A full `meta` atom should have the following:
	//
	// Version (1)
	// Flags (3)
	//
	// However, it's possible that it is written as a normal atom,
	// meaning this would be the size of the next atom.
	let _version_flags = data.read_u32::<BigEndian>()?;

	// Check if the next four bytes is one of the nested `meta` atoms
	let mut possible_ident = [0; 4];
	data.read_exact(&mut possible_ident)?;

	match &possible_ident {
		b"hdlr" | b"ilst" | b"mhdr" | b"ctry" | b"lang" => {
			data.seek(SeekFrom::Start(meta_pos))?;
			Ok(false)
		},
		_ => {
			data.seek(SeekFrom::Start(meta_pos + 4))?;
			Ok(true)
		},
	}
}
