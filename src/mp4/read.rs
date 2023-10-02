use super::atom_info::{AtomIdent, AtomInfo};
use super::moov::Moov;
use super::properties::Mp4Properties;
use super::Mp4File;
use crate::error::{ErrorKind, LoftyError, Result};
use crate::macros::{decode_err, err};
use crate::probe::ParseOptions;
use crate::traits::SeekStreamLen;

use std::io::{Read, Seek, SeekFrom};

use byteorder::{BigEndian, ReadBytesExt};

pub(super) struct AtomReader<R>
where
	R: Read + Seek,
{
	reader: R,
	start: u64,
	remaining_size: u64,
	len: u64,
}

impl<R> AtomReader<R>
where
	R: Read + Seek,
{
	pub(super) fn new(mut reader: R) -> Result<Self> {
		#[allow(unstable_name_collisions)]
		let len = reader.stream_len()?;
		Ok(Self {
			reader,
			start: 0,
			remaining_size: len,
			len,
		})
	}

	pub(super) fn reset_bounds(&mut self, start_position: u64, len: u64) {
		self.start = start_position;
		self.remaining_size = len;
		self.len = len;
	}

	pub(super) fn read_u8(&mut self) -> std::io::Result<u8> {
		self.remaining_size = self.remaining_size.saturating_sub(1);
		self.reader.read_u8()
	}

	pub(super) fn read_u16(&mut self) -> std::io::Result<u16> {
		self.remaining_size = self.remaining_size.saturating_sub(2);
		self.reader.read_u16::<BigEndian>()
	}

	pub(super) fn read_u32(&mut self) -> std::io::Result<u32> {
		self.remaining_size = self.remaining_size.saturating_sub(4);
		self.reader.read_u32::<BigEndian>()
	}

	pub(super) fn read_u64(&mut self) -> std::io::Result<u64> {
		self.remaining_size = self.remaining_size.saturating_sub(8);
		self.reader.read_u64::<BigEndian>()
	}

	pub(super) fn read_uint(&mut self, size: usize) -> std::io::Result<u64> {
		self.remaining_size = self.remaining_size.saturating_sub(size as u64);
		self.reader.read_uint::<BigEndian>(size)
	}

	pub(super) fn next(&mut self) -> Result<AtomInfo> {
		if self.remaining_size < 8 {
			err!(SizeMismatch);
		}

		AtomInfo::read(self, self.remaining_size)
	}

	pub(super) fn into_inner(self) -> R {
		self.reader
	}
}

impl<R> Seek for AtomReader<R>
where
	R: Read + Seek,
{
	fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
		match pos {
			SeekFrom::Start(s) => {
				if s > self.len {
					self.remaining_size = 0;

					let bound_end = self.start + self.len;
					return self.reader.seek(SeekFrom::Start(bound_end));
				}

				let ret = self.reader.seek(SeekFrom::Start(self.start + s))?;
				self.remaining_size = self.len.saturating_sub(ret);
				Ok(ret)
			},
			SeekFrom::End(s) => {
				if s >= 0 {
					self.remaining_size = 0;
					return self.reader.seek(SeekFrom::Start(self.start + self.len));
				}

				let bound_end = self.start + self.len;
				let relative_seek_count = core::cmp::min(self.len, s.unsigned_abs());
				self.reader.seek(SeekFrom::Start(
					bound_end.saturating_sub(relative_seek_count),
				))
			},
			SeekFrom::Current(s) => {
				if s.is_negative() {
					self.remaining_size = self.remaining_size.saturating_add(s.unsigned_abs());
				} else {
					self.remaining_size = self.remaining_size.saturating_sub(s as u64);
				}

				self.reader.seek(pos)
			},
		}
	}
}

impl<R> Read for AtomReader<R>
where
	R: Read + Seek,
{
	fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
		if self.remaining_size == 0 {
			return Ok(0);
		}

		let r = self.reader.read(buf)?;
		self.remaining_size = self.remaining_size.saturating_sub(r as u64);

		Ok(r)
	}
}

pub(in crate::mp4) fn verify_mp4<R>(reader: &mut AtomReader<R>) -> Result<String>
where
	R: Read + Seek,
{
	let atom = reader.next()?;

	if atom.ident != AtomIdent::Fourcc(*b"ftyp") {
		err!(UnknownFormat);
	}

	// size + identifier + major brand
	// There *should* be more, but this is all we need from it
	if atom.len < 12 {
		decode_err!(@BAIL Mp4, "\"ftyp\" atom too short");
	}

	let mut major_brand = vec![0; 4];
	reader.read_exact(&mut major_brand)?;

	reader.seek(SeekFrom::Current((atom.len - 12) as i64))?;

	String::from_utf8(major_brand)
		.map_err(|_| LoftyError::new(ErrorKind::BadAtom("Unable to parse \"ftyp\"'s major brand")))
}

#[allow(unstable_name_collisions)]
pub(crate) fn read_from<R>(data: &mut R, parse_options: ParseOptions) -> Result<Mp4File>
where
	R: Read + Seek,
{
	let mut reader = AtomReader::new(data)?;
	let file_length = reader.stream_len()?;

	let ftyp = verify_mp4(&mut reader)?;

	// Find the `moov` atom and restrict the reader to its length
	let moov_info = Moov::find(&mut reader)?;
	reader.reset_bounds(moov_info.start + 8, moov_info.len - 8);

	let moov = Moov::parse(
		&mut reader,
		parse_options.parsing_mode,
		parse_options.read_properties,
	)?;

	Ok(Mp4File {
		ftyp,
		ilst_tag: moov.meta,
		properties: if parse_options.read_properties {
			// Remove the length restriction
			reader.reset_bounds(0, file_length);
			super::properties::read_properties(&mut reader, &moov.traks, file_length)?
		} else {
			Mp4Properties::default()
		},
	})
}

pub(super) fn skip_unneeded<R>(reader: &mut R, ext: bool, len: u64) -> Result<()>
where
	R: Read + Seek,
{
	if ext {
		let pos = reader.stream_position()?;

		if let (pos, false) = pos.overflowing_add(len - 8) {
			reader.seek(SeekFrom::Start(pos))?;
		} else {
			err!(TooMuchData);
		}
	} else {
		reader.seek(SeekFrom::Current(i64::from(len as u32) - 8))?;
	}

	Ok(())
}

pub(super) fn nested_atom<R>(
	reader: &mut R,
	mut len: u64,
	expected: &[u8],
) -> Result<Option<AtomInfo>>
where
	R: Read + Seek,
{
	let mut ret = None;

	while len > 8 {
		let atom = AtomInfo::read(reader, len)?;

		match atom.ident {
			AtomIdent::Fourcc(ref fourcc) if fourcc == expected => {
				ret = Some(atom);
				break;
			},
			_ => {
				skip_unneeded(reader, atom.extended, atom.len)?;
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
) -> Result<(usize, Vec<AtomInfo>)>
where
	R: Read + Seek,
{
	let mut found_idx: usize = 0;
	let mut buf = Vec::new();

	let mut i = 0;

	while len > 8 {
		let atom = AtomInfo::read(reader, len)?;

		skip_unneeded(reader, atom.extended, atom.len)?;
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
			reader.seek(SeekFrom::Current(-8))?;
			Ok(false)
		},
		_ => {
			reader.seek(SeekFrom::Current(-4))?;
			Ok(true)
		},
	}
}
