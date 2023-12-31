use crate::error::Result;
use crate::mp4::atom_info::{AtomIdent, AtomInfo};
use crate::mp4::read::AtomReader;
use crate::probe::ParsingMode;

use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::ops::RangeBounds;

/// A wrapper around [`AtomInfo`] that allows us to track all of the children of containers we deem important
pub(super) struct ContextualAtom {
	info: AtomInfo,
	children: Vec<ContextualAtom>,
}

const IMPORTANT_CONTAINERS: [[u8; 4]; 4] = [*b"moov", *b"moof", *b"trak", *b"udta"];
impl ContextualAtom {
	pub(super) fn read<R>(
		reader: &mut R,
		reader_len: u64,
		parse_mode: ParsingMode,
	) -> Result<Option<ContextualAtom>>
	where
		R: Read + Seek,
	{
		let Some(info) = AtomInfo::read(reader, reader_len, parse_mode)? else {
			return Ok(None);
		};

		let mut children = Vec::new();

		if let AtomIdent::Fourcc(fourcc) = info.ident {
			if IMPORTANT_CONTAINERS.contains(&fourcc) {
				let mut len = info.len;
				while len > 8 {
					let Some(child) = ContextualAtom::read(reader, len, parse_mode)? else {
						break;
					};

					len = len.saturating_sub(child.info.len);
					children.push(child);
				}
			}
		}

		Ok(Some(ContextualAtom { info, children }))
	}

	/// This finds all instances of the `expected` fourcc within the atom's children
	///
	/// If `recurse` is `true`, then this will also search the children's children, and so on.
	pub(super) fn find_all_children<'a>(
		&'a self,
		expected: &'a [u8],
		recurse: bool,
	) -> AtomFindAll<'_, std::slice::Iter<'_, ContextualAtom>> {
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
	contents: Cursor<Vec<u8>>,
	atoms: Vec<ContextualAtom>,
	parse_mode: ParsingMode,
}

impl AtomWriter {
	/// Create a new [`AtomWriter`]
	///
	/// NOTE: This will not parse `content` for atoms. If you need to do that, use [`AtomWriter::new_from_file`]
	pub(super) fn new(content: Vec<u8>, parse_mode: ParsingMode) -> Self {
		Self {
			contents: Cursor::new(content),
			atoms: Vec::new(),
			parse_mode,
		}
	}

	/// Create a new [`AtomWriter`]
	///
	/// This will read the entire file into memory, and parse its atoms.
	pub(super) fn new_from_file(file: &mut File, parse_mode: ParsingMode) -> Result<Self> {
		let mut contents = Cursor::new(Vec::new());
		file.read_to_end(contents.get_mut())?;

		let len = contents.get_ref().len() as u64;
		let mut atoms = Vec::new();
		while let Some(atom) = ContextualAtom::read(&mut contents, len, parse_mode)? {
			atoms.push(atom);
		}

		contents.rewind()?;

		Ok(Self {
			contents,
			atoms,
			parse_mode,
		})
	}

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

	pub(super) fn len(&self) -> usize {
		self.contents.get_ref().len()
	}

	/// Convert this [`AtomWriter`] into an [`AtomReader`]
	///
	/// This is meant to be used for functions expecting an [`AtomReader`], with the reader
	/// being disposed of soon after.
	///
	/// TODO: This is kind of a hack? Might be better expressed with a trait
	pub(super) fn as_reader(&mut self) -> AtomReader<&mut Cursor<Vec<u8>>> {
		let len = self.contents.get_ref().len() as u64;
		AtomReader::new_with_len(&mut self.contents, len, self.parse_mode)
	}

	pub(super) fn into_contents(self) -> Vec<u8> {
		self.contents.into_inner()
	}

	pub(super) fn save_to(&mut self, file: &mut File) -> Result<()> {
		file.rewind()?;
		file.set_len(0)?;
		file.write_all(self.contents.get_ref())?;

		Ok(())
	}
}

impl Seek for AtomWriter {
	fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
		self.contents.seek(pos)
	}
}

impl Read for AtomWriter {
	fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
		self.contents.read(buf)
	}
}

impl Write for AtomWriter {
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		self.contents.write(buf)
	}

	fn flush(&mut self) -> std::io::Result<()> {
		self.contents.flush()
	}
}

pub struct AtomFindAll<'a, I> {
	atoms: I,
	expected_fourcc: &'a [u8],
	recurse: bool,
	current_container: Option<Box<AtomFindAll<'a, I>>>,
}

impl<'a> Iterator for AtomFindAll<'a, std::slice::Iter<'a, ContextualAtom>> {
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
			let AtomIdent::Fourcc(ref fourcc) = atom.info.ident else {
				continue;
			};

			if fourcc == self.expected_fourcc {
				return Some(&atom.info);
			}

			if self.recurse {
				self.current_container = Some(Box::new(
					atom.find_all_children(self.expected_fourcc, self.recurse),
				));
			}
		}
	}
}
