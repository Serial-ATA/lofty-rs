use crate::config::{ParseOptions, ParsingMode};
use crate::error::Result;
use crate::id3::v2::tag::Id3v2Tag;
use crate::macros::{err, try_vec};
use crate::util::text::utf8_decode;

use std::io::{Read, Seek, SeekFrom, Take};
use std::marker::PhantomData;

use byteorder::{ByteOrder, ReadBytesExt};

pub(crate) const IFF_CHUNK_HEADER_SIZE: u32 = 8;

// <https://www-mmsp.ece.mcgill.ca/Documents/AudioFormats/AIFF/Docs/AIFF-1.3.pdf>:
//
// "the concatenation of four printable ASCII character in the range '
// ' (SP, 0x20) through '~' (0x7E). Spaces (0x20) cannot precede printing
// characters; trailing spaces are allowed. Control characters are forbidden."
#[allow(clippy::let_and_return)]
pub(crate) fn valid_fourcc(fourcc: [u8; 4]) -> bool {
	let mut has_spaces = false;
	fourcc.iter().all(|b| {
		if *b == 0x20 {
			has_spaces = true;
		}

		let valid_char = (0x20..=0x7E).contains(b) && (!has_spaces || *b == 0x20);
		valid_char
	})
}

/// An IFF chunk reader
pub(crate) struct Chunks<R, B> {
	total_size: u64,
	remaining_size: u64,
	current_chunk_size: u32,
	current_chunk_remaining_size: u32,
	lock_state: Option<(u64, u32)>,
	reader: R,
	_phantom: PhantomData<B>,
}

impl<R: Read + Seek, B: ByteOrder> Chunks<R, B> {
	#[must_use]
	pub const fn new(reader: R, file_size: u64) -> Self {
		Self {
			total_size: file_size,
			remaining_size: file_size,
			current_chunk_size: 0,
			current_chunk_remaining_size: 0,
			lock_state: None,
			reader,
			_phantom: PhantomData,
		}
	}

	pub fn stream_position(&self) -> u64 {
		self.total_size - self.remaining_size
	}

	pub fn into_inner(self) -> R {
		self.reader
	}

	/// Get the next chunk in the stream, if it exists
	///
	/// This will return `Ok(None)` on EOF.
	pub fn next(&mut self, parse_mode: ParsingMode) -> Result<Option<Chunk<'_, R>>> {
		self.skip()?;

		if self.remaining_size < u64::from(IFF_CHUNK_HEADER_SIZE) {
			return Ok(None);
		}

		let start_pos = self.total_size - self.remaining_size;

		let mut fourcc = [0; 4];
		self.reader.read_exact(&mut fourcc)?;

		// Maybe we're eating into some junk? just assume the rest of the stream is useless
		if !valid_fourcc(fourcc) {
			log::warn!("Encountered invalid FourCC, stopping");

			self.remaining_size = 0;

			if parse_mode == ParsingMode::Strict {
				err!(SizeMismatch);
			}

			return Ok(None);
		}

		let size = self.reader.read_u32::<B>()?;
		if u64::from(size) > self.remaining_size {
			log::warn!("Chunk exceeds reader size, stopping");

			self.remaining_size = 0;

			if parse_mode == ParsingMode::Strict {
				err!(SizeMismatch);
			}

			return Ok(None);
		}

		self.remaining_size -= u64::from(IFF_CHUNK_HEADER_SIZE);
		self.current_chunk_size = size;
		self.current_chunk_remaining_size = size;

		Ok(Some(Chunk {
			file_remaining_size: &mut self.remaining_size,
			chunk_remaining_size: &mut self.current_chunk_remaining_size,
			start_pos,
			fourcc,
			size,
			reader: self.reader.by_ref().take(u64::from(size)),
		}))
	}

	/// Skip the rest of the current chunk's content
	pub fn skip(&mut self) -> Result<()> {
		if self.current_chunk_remaining_size > 0 {
			self.reader.seek(SeekFrom::Current(i64::from(
				self.current_chunk_remaining_size,
			)))?;

			self.remaining_size = self
				.remaining_size
				.saturating_sub(u64::from(self.current_chunk_remaining_size));

			self.current_chunk_remaining_size = 0;
		}

		self.correct_position(self.current_chunk_size)?;
		self.current_chunk_size = 0;

		Ok(())
	}

	/// Locks the chunks reader to the boundaries of the current chunk.
	///
	/// # Panics
	///
	/// Panics if the reader is already locked
	pub fn lock(&mut self) {
		assert!(self.lock_state.is_none(), "Chunks reader is already locked");

		let outer_remaining = self
			.remaining_size
			.saturating_sub(u64::from(self.current_chunk_remaining_size));

		self.lock_state = Some((outer_remaining, self.current_chunk_size));

		self.remaining_size = u64::from(self.current_chunk_remaining_size);
		self.current_chunk_size = 0;
		self.current_chunk_remaining_size = 0;
	}

	/// Unlocks the chunks reader from the current locked chunk.
	///
	/// This will skip any remaining unread content in the locked scope,
	/// handle parent padding, and restore the reader to the outer scope.
	pub fn unlock(&mut self) -> Result<()> {
		let Some((outer_remaining, parent_chunk_size)) = self.lock_state.take() else {
			return Ok(());
		};

		self.skip()?;

		if self.remaining_size > 0 {
			self.reader
				.seek(SeekFrom::Current(self.remaining_size as i64))?;
		}

		self.remaining_size = outer_remaining;
		self.current_chunk_size = 0;
		self.current_chunk_remaining_size = 0;

		self.correct_position(parent_chunk_size)?;

		Ok(())
	}

	fn correct_position(&mut self, current_chunk_size: u32) -> Result<()> {
		// Chunks are expected to start on even boundaries, and are padded
		// with a 0 if necessary. This is NOT the null terminator of the value,
		// and it is NOT included in the chunk's size

		let mut padding_size = 1;
		if current_chunk_size.is_multiple_of(2) || self.remaining_size < padding_size {
			return Ok(());
		}

		let padding_byte = self.reader.read_u8()?;

		// Unfortunately, not all encoders get padding bytes correct and include them in the chunk
		// size, so we need to check if we're eating into another chunk.
		if padding_byte != 0 && self.remaining_size > 4 {
			let mut next_fourcc = [padding_byte, 0, 0, 0];
			self.reader.read_exact(&mut next_fourcc[1..])?;

			let _ = next_fourcc.escape_ascii();
			let seek_back;
			if valid_fourcc(next_fourcc) {
				// Eating into the next chunk, so assume the padding was already consumed
				seek_back = -4;
				padding_size = 0;
			} else {
				// Maybe the encoder wrote a junk padding byte? or the next chunk fourcc is junk?
				// who knows. will probably error later.
				seek_back = -3;
			}

			self.reader.seek(SeekFrom::Current(seek_back))?;
		}

		self.remaining_size = self.remaining_size.saturating_sub(padding_size);

		Ok(())
	}
}

pub(crate) struct Chunk<'a, R> {
	file_remaining_size: &'a mut u64,
	chunk_remaining_size: &'a mut u32,
	start_pos: u64,
	pub fourcc: [u8; 4],
	size: u32,
	reader: Take<&'a mut R>,
}

impl<R: Read + Seek> Chunk<'_, R> {
	/// Get the size of the chunk
	///
	/// This does **not** include the size of the chunk header.
	pub fn size(&self) -> u32 {
		self.size
	}

	/// Get the start position of the chunk
	pub fn start(&self) -> u64 {
		self.start_pos
	}

	/// Read a C-style string from the chunk
	pub fn read_cstring(&mut self) -> Result<String> {
		let cont = self.content()?;
		utf8_decode(cont)
	}

	/// Read a UTF-8 string from the chunk
	///
	/// If `size` isn't provided, the string is assumed to take up the entire chunk's content.
	pub fn read_string(&mut self, size: Option<u32>) -> Result<String> {
		let size = size.map_or(self.size() as usize, |size| size as usize);

		let mut content = try_vec![0; size];
		self.read_exact(&mut content)?;

		utf8_decode(content)
	}

	/// Read the entire chunk's content
	pub fn content(&mut self) -> Result<Vec<u8>> {
		let mut content = try_vec![0; self.size() as usize];
		self.read_exact(&mut content)?;
		Ok(content)
	}

	/// Parse this chunk as an ID3v2 tag
	pub fn id3_chunk(&mut self, parse_options: ParseOptions) -> Result<Option<Id3v2Tag>> {
		use crate::id3::v2::header::Id3v2Header;
		use crate::id3::v2::read::parse_id3v2;

		let content = self.content()?;

		if content.len() < 10 {
			log::warn!("ID3 chunk too small to contain a valid header");
			if parse_options.parsing_mode == crate::config::ParsingMode::Strict {
				err!(FakeTag);
			}
			return Ok(None);
		}

		let reader = &mut &*content;

		let header = Id3v2Header::parse(reader)?;
		let id3v2 = parse_id3v2(reader, header, parse_options)?;

		Ok(Some(id3v2))
	}
}

impl<R: Read> Read for Chunk<'_, R> {
	fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
		let bytes_read = self.reader.read(buf)?;
		*self.file_remaining_size = self.file_remaining_size.saturating_sub(bytes_read as u64);
		*self.chunk_remaining_size = self.chunk_remaining_size.saturating_sub(bytes_read as u32);
		Ok(bytes_read)
	}
}

impl<R: Seek> Seek for Chunk<'_, R> {
	fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
		let old_limit = self.reader.limit();
		let ret = self.reader.seek(pos)?;
		let new_limit = self.reader.limit();

		let delta = (old_limit as i64) - (new_limit as i64);

		*self.file_remaining_size = (*self.file_remaining_size as i64 - delta) as u64;
		*self.chunk_remaining_size = (i64::from(*self.chunk_remaining_size) - delta) as u32;

		Ok(ret)
	}
}
