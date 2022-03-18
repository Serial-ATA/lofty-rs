use crate::error::Result;
use crate::macros::try_vec;

use std::io::{Read, Seek, SeekFrom};

use byteorder::{BigEndian, ReadBytesExt};

pub(crate) struct Block {
	pub(crate) byte: u8,
	pub(crate) ty: u8,
	pub(crate) last: bool,
	pub(crate) content: Vec<u8>,
	pub(crate) start: u64,
	pub(crate) end: u64,
}

impl Block {
	pub(crate) fn read<R>(data: &mut R) -> Result<Self>
	where
		R: Read + Seek,
	{
		let start = data.seek(SeekFrom::Current(0))?;

		let byte = data.read_u8()?;
		let last = (byte & 0x80) != 0;
		let ty = byte & 0x7F;

		let size = data.read_uint::<BigEndian>(3)? as u32;

		let mut content = try_vec![0; size as usize];
		data.read_exact(&mut content)?;

		let end = data.seek(SeekFrom::Current(0))?;

		Ok(Self {
			byte,
			ty,
			last,
			content,
			start,
			end,
		})
	}
}
