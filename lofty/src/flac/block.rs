#![allow(dead_code)]

use crate::error::Result;
use crate::macros::try_vec;

use std::io::{Read, Seek, SeekFrom};

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
	pub(crate) fn read<R, P>(data: &mut R, mut predicate: P) -> Result<Self>
	where
		R: Read + Seek,
		P: FnMut(u8) -> bool,
	{
		let start = data.stream_position()?;

		let byte = data.read_u8()?;
		let last = (byte & 0x80) != 0;
		let ty = byte & 0x7F;

		let size = data.read_u24::<BigEndian>()?;
		log::trace!("Reading FLAC block, type: {ty}, size: {size}");

		let mut content;
		if predicate(ty) {
			content = try_vec![0; size as usize];
			data.read_exact(&mut content)?;
		} else {
			content = Vec::new();
			data.seek(SeekFrom::Current(i64::from(size)))?;
		}

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
