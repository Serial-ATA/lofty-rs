use crate::error::Result;
use crate::macros::try_vec;

use std::io::{Read, Seek};

use byteorder::{BigEndian, ReadBytesExt};

pub(in crate::flac) const BLOCK_ID_STREAMINFO: u8 = 0;
pub(in crate::flac) const BLOCK_ID_PADDING: u8 = 1;
pub(in crate::flac) const BLOCK_ID_SEEKTABLE: u8 = 3;
pub(in crate::flac) const BLOCK_ID_VORBIS_COMMENTS: u8 = 4;
pub(in crate::flac) const BLOCK_ID_PICTURE: u8 = 6;

pub(crate) struct Block {
	pub(super) byte: u8,
	pub(super) ty: u8,
	pub(super) last: bool,
	pub(crate) content: Vec<u8>,
	pub(super) start: u64,
	pub(super) end: u64,
}

impl Block {
	pub(crate) fn read<R>(data: &mut R) -> Result<Self>
	where
		R: Read + Seek,
	{
		let start = data.stream_position()?;

		let byte = data.read_u8()?;
		let last = (byte & 0x80) != 0;
		let ty = byte & 0x7F;

		let size = data.read_u24::<BigEndian>()?;

		let mut content = try_vec![0; size as usize];
		data.read_exact(&mut content)?;

		let end = data.stream_position()?;

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
