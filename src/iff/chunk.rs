use crate::error::{ErrorKind, LoftyError, Result};
#[cfg(feature = "id3v2")]
use crate::id3::v2::read::parse_id3v2;
use crate::id3::v2::read_id3v2_header;
#[cfg(feature = "id3v2")]
use crate::id3::v2::tag::Id3v2Tag;
use crate::macros::try_vec;

use std::io::{Read, Seek, SeekFrom};
use std::marker::PhantomData;

use byteorder::{ByteOrder, ReadBytesExt};

pub(crate) struct Chunks<B>
where
	B: ByteOrder,
{
	pub fourcc: [u8; 4],
	pub size: u32,
	file_size: u32,
	_phantom: PhantomData<B>,
}

impl<B: ByteOrder> Chunks<B> {
	pub fn new(file_size: u32) -> Self {
		Self {
			fourcc: [0; 4],
			size: 0,
			file_size,
			_phantom: PhantomData,
		}
	}

	pub fn next<R>(&mut self, data: &mut R) -> Result<()>
	where
		R: Read,
	{
		data.read_exact(&mut self.fourcc)?;
		self.size = data.read_u32::<B>()?;

		Ok(())
	}

	pub fn read_cstring<R>(&mut self, data: &mut R) -> Result<String>
	where
		R: Read + Seek,
	{
		let cont = self.content(data)?;
		self.correct_position(data)?;

		let value_str = std::str::from_utf8(&cont)?;

		Ok(value_str.trim_end_matches('\0').to_string())
	}

	pub fn read_pstring<R>(&mut self, data: &mut R, size: Option<u32>) -> Result<String>
	where
		R: Read + Seek,
	{
		let cont = if let Some(size) = size {
			self.read(data, size)?
		} else {
			self.content(data)?
		};

		if cont.len() % 2 != 0 {
			data.seek(SeekFrom::Current(1))?;
		}

		Ok(String::from_utf8(cont)?)
	}

	pub fn content<R>(&mut self, data: &mut R) -> Result<Vec<u8>>
	where
		R: Read,
	{
		self.read(data, self.size)
	}

	fn read<R>(&self, data: &mut R, size: u32) -> Result<Vec<u8>>
	where
		R: Read,
	{
		if size + 4 > self.file_size {
			return Err(LoftyError::new(ErrorKind::TooMuchData));
		}

		let mut content = try_vec![0; size as usize];
		data.read_exact(&mut content)?;

		Ok(content)
	}

	#[cfg(feature = "id3v2")]
	pub fn id3_chunk<R>(&mut self, data: &mut R) -> Result<Id3v2Tag>
	where
		R: Read + Seek,
	{
		let mut value = vec![0; self.size as usize];
		data.read_exact(&mut value)?;

		let reader = &mut &*value;

		let header = read_id3v2_header(reader)?;
		let id3v2 = parse_id3v2(reader, header)?;

		// Skip over the footer
		if id3v2.flags().footer {
			data.seek(SeekFrom::Current(10))?;
		}

		self.correct_position(data)?;

		Ok(id3v2)
	}

	pub fn skip<R>(&mut self, data: &mut R) -> Result<()>
	where
		R: Read + Seek,
	{
		data.seek(SeekFrom::Current(i64::from(self.size)))?;
		self.correct_position(data)?;

		Ok(())
	}

	pub fn correct_position<R>(&mut self, data: &mut R) -> Result<()>
	where
		R: Read + Seek,
	{
		// Chunks are expected to start on even boundaries, and are padded
		// with a 0 if necessary. This is NOT the null terminator of the value,
		// and it is NOT included in the chunk's size
		if self.size % 2 != 0 {
			data.seek(SeekFrom::Current(1))?;
		}

		Ok(())
	}
}
