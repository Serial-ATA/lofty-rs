use super::atom::Atom;
use super::moov::Moov;
use super::properties::read_properties;
use super::Mp4File;
use crate::error::{LoftyError, Result};

use std::io::{Read, Seek, SeekFrom};

fn verify_mp4<R>(data: &mut R) -> Result<String>
where
	R: Read + Seek,
{
	let atom = Atom::read(data)?;

	if atom.ident != "ftyp" {
		return Err(LoftyError::UnknownFormat);
	}

	let mut major_brand = vec![0; 4];
	data.read_exact(&mut major_brand)?;

	data.seek(SeekFrom::Current((atom.len - 12) as i64))?;

	String::from_utf8(major_brand)
		.map_err(|_| LoftyError::BadAtom("Unable to parse \"ftyp\"'s major brand"))
}

#[allow(clippy::similar_names)]
pub(crate) fn read_from<R>(data: &mut R) -> Result<Mp4File>
where
	R: Read + Seek,
{
	let ftyp = verify_mp4(data)?;

	let mut moov = false;

	while let Ok(atom) = Atom::read(data) {
		if &*atom.ident == "moov" {
			moov = true;
			break;
		}

		skip_unneeded(data, atom.extended, atom.len)?;
	}

	if !moov {
		return Err(LoftyError::Mp4("No \"moov\" atom found"));
	}

	let moov = Moov::parse(data)?;

	Ok(Mp4File {
		ftyp,
		ilst: moov.meta,
		properties: read_properties(data, &moov.traks)?,
	})
}

pub(crate) fn skip_unneeded<R>(data: &mut R, ext: bool, len: u64) -> Result<()>
where
	R: Read + Seek,
{
	if ext {
		let pos = data.seek(SeekFrom::Current(0))?;

		if let (pos, false) = pos.overflowing_add(len - 8) {
			data.seek(SeekFrom::Start(pos))?;
		} else {
			return Err(LoftyError::TooMuchData);
		}
	} else {
		data.seek(SeekFrom::Current(i64::from(len as u32) - 8))?;
	}

	Ok(())
}

pub(crate) fn nested_atom<R>(data: &mut R, len: u64, expected: &str) -> Result<Option<Atom>>
where
	R: Read + Seek,
{
	let mut read = 8;
	let mut ret = None;

	while read < len {
		let atom = Atom::read(data)?;

		match &*atom.ident {
			ident if ident == expected => {
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
