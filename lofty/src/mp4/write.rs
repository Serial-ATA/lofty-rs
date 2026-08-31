use crate::config::ParsingMode;
use crate::error::{FileEncodingError, FileParseError};
use crate::mp4::atom_info::{ATOM_HEADER_LEN, AtomIdent, AtomInfo, IDENTIFIER_LEN};
use crate::mp4::error::{AtomParseError, Mp4ParseError};
use crate::mp4::read::{meta_is_full, skip_atom};
use crate::util::io::FileLike;

use std::cell::{RefCell, RefMut};
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::ops::{Bound, RangeBounds};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

/// A wrapper around [`AtomInfo`] that allows us to track all of the children of containers we deem important
#[derive(Debug)]
pub(super) struct ContextualAtom {
	pub(crate) info: AtomInfo,
	pub(crate) children: Vec<ContextualAtom>,
}

const META_ATOM_IDENT: AtomIdent<'_> = AtomIdent::Fourcc(*b"meta");

#[rustfmt::skip]
const IMPORTANT_CONTAINERS: &[[u8; 4]] = &[
	*b"moov",
		*b"udta",
		*b"moof",
		*b"trak",
			*b"mdia",
				*b"minf",
					*b"stbl",
];
impl ContextualAtom {
	pub(super) fn read<R>(
		reader: &mut R,
		reader_len: &mut u64,
		parse_mode: ParsingMode,
	) -> Result<Option<ContextualAtom>, AtomParseError>
	where
		R: Read + Seek,
	{
		if *reader_len == 0 {
			return Ok(None);
		}

		let Some(info) = AtomInfo::read(reader, *reader_len, parse_mode)? else {
			return Ok(None);
		};

		match info.ident {
			AtomIdent::Fourcc(ident) if IMPORTANT_CONTAINERS.contains(&ident) => {},
			_ => {
				*reader_len = reader_len.saturating_sub(info.len);

				// We don't care about the atom's contents
				skip_atom(reader, info.extended, info.len)?;
				return Ok(Some(ContextualAtom {
					info,
					children: Vec::new(),
				}));
			},
		}

		let mut len = info.len - info.header_size();
		let mut children = Vec::new();

		// See meta_is_full for details
		if info.ident == META_ATOM_IDENT && meta_is_full(reader)? {
			len -= 4;
		}

		while let Some(child) = Self::read(reader, &mut len, parse_mode)? {
			children.push(child);
		}

		if len != 0 {
			return Err(AtomParseError::message(
				Some(info.ident),
				"unable to read entire container",
			));
		}

		*reader_len = reader_len.saturating_sub(info.len);
		// reader.seek(SeekFrom::Current(*reader_len as i64))?; // Skip any remaining bytes
		Ok(Some(ContextualAtom { info, children }))
	}

	/// This finds all instances of the `expected` fourcc within the atom's children
	///
	/// If `recurse` is `true`, then this will also search the children's children, and so on.
	pub(super) fn find_all_children(
		&self,
		expected: [u8; 4],
		recurse: bool,
	) -> AtomFindAll<std::slice::Iter<'_, ContextualAtom>> {
		AtomFindAll {
			atoms: self.children.iter(),
			expected_fourcc: expected,
			recurse,
			current_container: None,
		}
	}
}

pub(super) struct ContextualAtoms(Vec<ContextualAtom>);

impl ContextualAtoms {
	pub(super) fn find_atom(&self, fourcc: [u8; 4]) -> Option<&ContextualAtom> {
		self.0
			.iter()
			.find(|atom| matches!(atom.info.ident, AtomIdent::Fourcc(ident) if ident == fourcc))
	}
}

/// This is a simple wrapper around a [`Cursor`] that allows us to store additional atom information
///
/// The `atoms` field contains all of the atoms within the file, with containers deemed important (see `IMPORTANT_CONTAINERS`)
/// being parsed recursively. We are then able to use this information to find atoms nested deeply within the file.
///
/// Atoms that are not "important" containers are simply parsed at the top level, with all children being skipped.
pub(super) struct AtomWriter {
	contents: RefCell<Cursor<Vec<u8>>>,
	atoms: ContextualAtoms,
}

impl AtomWriter {
	/// Create a new [`AtomWriter`]
	///
	/// NOTE: This will not parse `content` for atoms. If you need to do that, use [`AtomWriter::new_from_file`]
	pub(super) fn new(content: Vec<u8>, _parse_mode: ParsingMode) -> Self {
		Self {
			contents: RefCell::new(Cursor::new(content)),
			atoms: ContextualAtoms(Vec::new()),
		}
	}

	/// Create a new [`AtomWriter`]
	///
	/// This will read the entire file into memory, and parse its atoms.
	pub(super) fn new_from_file<F>(
		file: &mut F,
		parse_mode: ParsingMode,
	) -> Result<Self, Mp4ParseError>
	where
		F: FileLike,
	{
		let mut contents = Cursor::new(Vec::new());
		file.read_to_end(contents.get_mut())?;

		let mut len = contents.get_ref().len() as u64;
		let mut atoms = Vec::new();
		while let Some(atom) = ContextualAtom::read(&mut contents, &mut len, parse_mode)? {
			atoms.push(atom);
		}

		contents.rewind()?;

		Ok(Self {
			contents: RefCell::new(contents),
			atoms: ContextualAtoms(atoms),
		})
	}

	pub(super) fn atoms(&self) -> &ContextualAtoms {
		&self.atoms
	}

	pub(super) fn into_contents(self) -> Vec<u8> {
		self.contents.into_inner().into_inner()
	}

	/// Start a write operation
	///
	/// # Panics
	///
	/// This will panic if a write is already active. Any previous handle **must** be dropped.
	pub(super) fn start_write(&self) -> AtomWriterCompanion<'_> {
		let contents = self.contents.borrow_mut();
		let original_length = contents.get_ref().len();
		AtomWriterCompanion {
			shift_pos: None,
			original_length,
			atoms: self.atoms(),
			contents,
			finished: false,
		}
	}

	pub(super) fn save_to<F>(&mut self, file: &mut F) -> Result<(), FileEncodingError>
	where
		F: FileLike,
	{
		file.rewind()?;
		file.truncate(0)?;
		file.write_all(self.contents.borrow().get_ref())?;

		Ok(())
	}
}

/// The actual handler of the writing operations
///
/// NOTE: The writer **must** be `finish()`ed before dropping
pub(super) struct AtomWriterCompanion<'a> {
	shift_pos: Option<u64>,
	original_length: usize,
	atoms: &'a ContextualAtoms,
	contents: RefMut<'a, Cursor<Vec<u8>>>,
	finished: bool,
}

impl AtomWriterCompanion<'_> {
	/// Insert a byte at the given index
	///
	/// NOTE: This will not affect the position of the inner [`Cursor`]
	pub(super) fn insert(&mut self, index: usize, byte: u8) {
		let index = index as u64;
		self.shift_pos = Some(self.shift_pos.map_or(index, |p| std::cmp::min(p, index)));
		self.contents.get_mut().insert(index as usize, byte);
	}

	/// Replace the contents of the given range
	pub(super) fn splice<R, I>(&mut self, range: R, replacement: I)
	where
		R: RangeBounds<usize>,
		I: IntoIterator<Item = u8>,
	{
		let lower_bound = match range.start_bound() {
			Bound::Included(&bound) => bound as u64,
			Bound::Excluded(&bound) => (bound + 1) as u64,
			Bound::Unbounded => 0,
		};

		self.shift_pos = Some(
			self.shift_pos
				.map_or(lower_bound, |p| std::cmp::min(p, lower_bound)),
		);
		self.contents.get_mut().splice(range, replacement);
	}

	/// Write an atom's size
	///
	/// NOTES:
	/// * This expects the cursor to be at the start of the atom size
	/// * This will leave the cursor at the start of the atom's data
	pub(super) fn write_atom_size(
		&mut self,
		start: u64,
		size: u64,
		extended: bool,
	) -> std::io::Result<()> {
		if u32::try_from(size).is_ok() {
			// ???? (identifier)
			self.write_u32::<BigEndian>(size as u32)?;
			self.seek(SeekFrom::Current(IDENTIFIER_LEN as i64))?;
			return Ok(());
		}

		// 64-bit extended size
		// 0001 (identifier) ????????

		// Extended size indicator
		self.write_u32::<BigEndian>(1)?;
		// Skip identifier
		self.seek(SeekFrom::Current(IDENTIFIER_LEN as i64))?;

		let extended_size = size.to_be_bytes();

		if extended {
			// Overwrite existing extended size
			self.write_u64::<BigEndian>(size)?;
		} else {
			for (index, b) in extended_size.into_iter().enumerate() {
				self.insert((start + 8 + index as u64) as usize, b);
			}

			self.seek(SeekFrom::Current(8))?;
		}

		Ok(())
	}

	pub(super) fn len(&self) -> usize {
		self.contents.get_ref().len()
	}

	/// Finishes the write operation and updates offset atoms if needed
	pub(super) fn finish(mut self) -> Result<(), FileEncodingError> {
		self.finished = true;
		self.update_offsets()
	}

	/// Update `moov` offset atoms, if needed
	///
	/// This updates:
	///
	/// * `moov.stco`
	/// * `moov.co64`
	/// * `moov.moof.tfhd`
	///
	/// Whenever a write handle is finished.
	fn update_offsets(&mut self) -> Result<(), FileEncodingError> {
		let Some(shift_pos) = self.shift_pos else {
			// No edits were made
			return Ok(());
		};

		let difference = self.contents.get_ref().len() as i64 - self.original_length as i64;
		if difference == 0 {
			// Contents didn't shift at all
			return Ok(());
		}

		let Some(moov) = self.atoms.find_atom(*b"moov") else {
			return Ok(());
		};

		log::debug!("Checking for offset atoms to update");

		// 32-bit offsets
		for stco in moov.find_all_children(*b"stco", true) {
			log::trace!("Found `stco` atom");

			let mut stco_start = stco.start;
			if stco.extended {
				return Err(FileParseError::from(AtomParseError::message(
					Some(stco.ident.clone()),
					"found an extended `stco` atom",
				))
				.into());
			}

			if stco_start >= shift_pos {
				stco_start = (stco_start as i64 + difference) as u64;
			}

			self.seek(SeekFrom::Start(stco_start + ATOM_HEADER_LEN + 4))?;

			let count = self.read_u32::<BigEndian>()?;
			for _ in 0..count {
				let read_offset = self.read_u32::<BigEndian>()?;
				if u64::from(read_offset) < shift_pos {
					continue;
				}
				self.seek(SeekFrom::Current(-4))?;
				self.write_u32::<BigEndian>((i64::from(read_offset) + difference) as u32)?;

				log::trace!(
					"Updated offset from {read_offset} to {}",
					(i64::from(read_offset) + difference) as u32
				);
			}
		}

		// 64-bit offsets
		for co64 in moov.find_all_children(*b"co64", true) {
			log::trace!("Found `co64` atom");

			let mut co64_start = co64.start;
			if co64_start >= shift_pos {
				co64_start = (co64_start as i64 + difference) as u64;
			}

			self.seek(SeekFrom::Start(co64_start + ATOM_HEADER_LEN + 8 + 4))?;

			let count = self.read_u32::<BigEndian>()?;
			for _ in 0..count {
				let read_offset = self.read_u64::<BigEndian>()?;
				if read_offset < shift_pos {
					continue;
				}

				self.seek(SeekFrom::Current(-8))?;
				self.write_u64::<BigEndian>((read_offset as i64 + difference) as u64)?;

				log::trace!(
					"Updated offset from {read_offset} to {}",
					((read_offset as i64) + difference) as u64
				);
			}
		}

		let Some(moof) = self.atoms.find_atom(*b"moof") else {
			return Ok(());
		};

		log::trace!("Found `moof` atom, checking for `tfhd` atoms to update");

		// 64-bit offsets
		for tfhd in moof.find_all_children(*b"tfhd", true) {
			log::trace!("Found `tfhd` atom");

			let mut tfhd_start = tfhd.start;
			if tfhd.extended {
				return Err(FileParseError::from(AtomParseError::message(
					Some(tfhd.ident.clone()),
					"found an extended `tfhd` atom",
				))
				.into());
			}

			if tfhd_start >= shift_pos {
				tfhd_start = (tfhd_start as i64 + difference) as u64;
			}

			// Skip atom header + version (1)
			self.seek(SeekFrom::Start(tfhd_start + ATOM_HEADER_LEN + 1))?;

			let flags = self.read_u24::<BigEndian>()?;
			let base_data_offset = (flags & 0b1) != 0;

			if base_data_offset {
				let read_offset = self.read_u64::<BigEndian>()?;
				if read_offset < shift_pos {
					continue;
				}

				self.seek(SeekFrom::Current(-8))?;
				self.write_u64::<BigEndian>((read_offset as i64 + difference) as u64)?;

				log::trace!(
					"Updated offset from {read_offset} to {}",
					((read_offset as i64) + difference) as u64
				);
			}
		}

		Ok(())
	}
}

impl Drop for AtomWriterCompanion<'_> {
	fn drop(&mut self) {
		assert!(
			self.finished || std::thread::panicking(),
			"`AtomWriterCompanion` was not finished"
		);
	}
}

impl Seek for AtomWriterCompanion<'_> {
	fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
		self.contents.seek(pos)
	}
}

impl Read for AtomWriterCompanion<'_> {
	fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
		self.contents.read(buf)
	}
}

impl Write for AtomWriterCompanion<'_> {
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		self.contents.write(buf)
	}

	fn flush(&mut self) -> std::io::Result<()> {
		self.contents.flush()
	}
}

pub struct AtomFindAll<I> {
	atoms: I,
	expected_fourcc: [u8; 4],
	recurse: bool,
	current_container: Option<Box<AtomFindAll<I>>>,
}

impl<'a> Iterator for AtomFindAll<std::slice::Iter<'a, ContextualAtom>> {
	type Item = &'a AtomInfo;

	fn next(&mut self) -> Option<Self::Item> {
		if let Some(ref mut container) = self.current_container {
			match container.next() {
				Some(next) => {
					return Some(next);
				},
				None => {
					self.current_container = None;
				},
			}
		}

		loop {
			let atom = self.atoms.next()?;
			let AtomIdent::Fourcc(fourcc) = atom.info.ident else {
				continue;
			};

			if fourcc == self.expected_fourcc {
				return Some(&atom.info);
			}

			if self.recurse {
				if atom.children.is_empty() {
					continue;
				}

				self.current_container = Some(Box::new(
					atom.find_all_children(self.expected_fourcc, self.recurse),
				));

				return self.next();
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	#[should_panic(expected = "`AtomWriterCompanion` was not finished")]
	fn unfinished_companion_panics() {
		let writer = AtomWriter::new(Vec::new(), ParsingMode::Strict);
		let _write_handle = writer.start_write();
	}

	#[test]
	fn finished_companion_succeeds() {
		let writer = AtomWriter::new(Vec::new(), ParsingMode::Strict);
		let write_handle = writer.start_write();
		assert!(write_handle.finish().is_ok());
	}
}
