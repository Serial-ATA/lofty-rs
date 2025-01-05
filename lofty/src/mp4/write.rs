use crate::config::ParsingMode;
use crate::error::{LoftyError, Result};
use crate::io::{FileLike, Length, Truncate};
use crate::macros::err;
use crate::mp4::atom_info::{AtomIdent, AtomInfo, IDENTIFIER_LEN};
use crate::mp4::read::{meta_is_full, skip_atom};

use std::cell::{RefCell, RefMut};
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::ops::RangeBounds;

use byteorder::{BigEndian, WriteBytesExt};

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
	) -> Result<Option<ContextualAtom>>
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
			// TODO: Print the container ident
			err!(BadAtom("Unable to read entire container"));
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

/// This is a simple wrapper around a [`Cursor`] that allows us to store additional atom information
///
/// The `atoms` field contains all of the atoms within the file, with containers deemed important (see `IMPORTANT_CONTAINERS`)
/// being parsed recursively. We are then able to use this information to find atoms nested deeply within the file.
///
/// Atoms that are not "important" containers are simply parsed at the top level, with all children being skipped.
pub(super) struct AtomWriter {
	contents: RefCell<Cursor<Vec<u8>>>,
	atoms: Vec<ContextualAtom>,
}

impl AtomWriter {
	/// Create a new [`AtomWriter`]
	///
	/// NOTE: This will not parse `content` for atoms. If you need to do that, use [`AtomWriter::new_from_file`]
	pub(super) fn new(content: Vec<u8>, _parse_mode: ParsingMode) -> Self {
		Self {
			contents: RefCell::new(Cursor::new(content)),
			atoms: Vec::new(),
		}
	}

	/// Create a new [`AtomWriter`]
	///
	/// This will read the entire file into memory, and parse its atoms.
	pub(super) fn new_from_file<F>(file: &mut F, parse_mode: ParsingMode) -> Result<Self>
	where
		F: FileLike,
		LoftyError: From<<F as Truncate>::Error>,
		LoftyError: From<<F as Length>::Error>,
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
			atoms,
		})
	}

	pub(super) fn find_contextual_atom(&self, fourcc: [u8; 4]) -> Option<&ContextualAtom> {
		self.atoms
			.iter()
			.find(|atom| matches!(atom.info.ident, AtomIdent::Fourcc(ident) if ident == fourcc))
	}

	pub(super) fn into_contents(self) -> Vec<u8> {
		self.contents.into_inner().into_inner()
	}

	pub(super) fn start_write(&self) -> AtomWriterCompanion<'_> {
		AtomWriterCompanion {
			contents: self.contents.borrow_mut(),
		}
	}

	pub(super) fn save_to<F>(&mut self, file: &mut F) -> Result<()>
	where
		F: FileLike,
		LoftyError: From<<F as Truncate>::Error>,
		LoftyError: From<<F as Length>::Error>,
	{
		file.rewind()?;
		file.truncate(0)?;
		file.write_all(self.contents.borrow().get_ref())?;

		Ok(())
	}
}

/// The actual handler of the writing operations
pub(super) struct AtomWriterCompanion<'a> {
	contents: RefMut<'a, Cursor<Vec<u8>>>,
}

impl AtomWriterCompanion<'_> {
	/// Insert a byte at the given index
	///
	/// NOTE: This will not affect the position of the inner [`Cursor`]
	pub(super) fn insert(&mut self, index: usize, byte: u8) {
		self.contents.get_mut().insert(index, byte);
	}

	/// Replace the contents of the given range
	pub(super) fn splice<R, I>(&mut self, range: R, replacement: I)
	where
		R: RangeBounds<usize>,
		I: IntoIterator<Item = u8>,
	{
		self.contents.get_mut().splice(range, replacement);
	}

	/// Write an atom's size
	///
	/// NOTES:
	/// * This expects the cursor to be at the start of the atom size
	/// * This will leave the cursor at the start of the atom's data
	pub(super) fn write_atom_size(&mut self, start: u64, size: u64, extended: bool) -> Result<()> {
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
			for i in extended_size {
				self.insert((start + 8 + u64::from(i)) as usize, i);
			}

			self.seek(SeekFrom::Current(8))?;
		}

		Ok(())
	}

	pub(super) fn len(&self) -> usize {
		self.contents.get_ref().len()
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
