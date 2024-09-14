use crate::config::ParsingMode;
use crate::error::Result;
use crate::macros::err;
use crate::mp4::atom_info::AtomInfo;
use crate::util::io::SeekStreamLen;

use std::io::{Read, Seek, SeekFrom};

use byteorder::{BigEndian, ReadBytesExt};

/// A reader for an MP4 file
///
/// This is a special wrapper around a reader that provides:
///
/// * [`Self::next`] to read atoms.
/// * `read_u*` methods to read integers without needing to specify the endianness.
/// * Bounds checking on reads and seeks to prevent going outside the file.
pub(crate) struct AtomReader<R>
where
	R: Read + Seek,
{
	reader: R,
	start: u64,
	remaining_size: u64,
	len: u64,
	parse_mode: ParsingMode,
}

impl<R> AtomReader<R>
where
	R: Read + Seek,
{
	/// Create a new `AtomReader`
	pub(crate) fn new(mut reader: R, parse_mode: ParsingMode) -> Result<Self> {
		let len = reader.stream_len_hack()?;
		Ok(Self {
			reader,
			start: 0,
			remaining_size: len,
			len,
			parse_mode,
		})
	}

	/// Set new bounds for the reader
	///
	/// This is useful when reading an atom such as `moov`, where we only want to read it and its
	/// children. We can read the atom, set the bounds to the atom's length, and then read the children
	/// without worrying about reading past the atom's end.
	pub(crate) fn reset_bounds(&mut self, start_position: u64, len: u64) {
		self.start = start_position;
		self.remaining_size = len;
		self.len = len;
	}

	pub(crate) fn read_u8(&mut self) -> std::io::Result<u8> {
		self.remaining_size = self.remaining_size.saturating_sub(1);
		self.reader.read_u8()
	}

	pub(crate) fn read_u16(&mut self) -> std::io::Result<u16> {
		self.remaining_size = self.remaining_size.saturating_sub(2);
		self.reader.read_u16::<BigEndian>()
	}

	pub(crate) fn read_u24(&mut self) -> std::io::Result<u32> {
		self.remaining_size = self.remaining_size.saturating_sub(3);
		self.reader.read_u24::<BigEndian>()
	}

	pub(crate) fn read_u32(&mut self) -> std::io::Result<u32> {
		self.remaining_size = self.remaining_size.saturating_sub(4);
		self.reader.read_u32::<BigEndian>()
	}

	pub(crate) fn read_u64(&mut self) -> std::io::Result<u64> {
		self.remaining_size = self.remaining_size.saturating_sub(8);
		self.reader.read_u64::<BigEndian>()
	}

	pub(crate) fn read_uint(&mut self, size: usize) -> std::io::Result<u64> {
		self.remaining_size = self.remaining_size.saturating_sub(size as u64);
		self.reader.read_uint::<BigEndian>(size)
	}

	/// Read the next atom in the file
	///
	/// This will leave the reader at the beginning of the atom content.
	pub(crate) fn next(&mut self) -> Result<Option<AtomInfo>> {
		if self.remaining_size == 0 {
			return Ok(None);
		}

		if self.remaining_size < 8 {
			err!(SizeMismatch);
		}

		AtomInfo::read(self, self.remaining_size, self.parse_mode)
	}

	pub(crate) fn into_inner(self) -> R {
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
