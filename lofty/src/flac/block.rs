#![allow(dead_code)]

use crate::error::Result;
use crate::macros::{err, try_vec};
use crate::picture::{Picture, PictureInformation};

use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};

pub(in crate::flac) const BLOCK_ID_STREAMINFO: u8 = 0;
pub(in crate::flac) const BLOCK_ID_PADDING: u8 = 1;
pub(in crate::flac) const BLOCK_ID_SEEKTABLE: u8 = 3;
pub(in crate::flac) const BLOCK_ID_VORBIS_COMMENTS: u8 = 4;
pub(in crate::flac) const BLOCK_ID_PICTURE: u8 = 6;

const BLOCK_HEADER_SIZE: u64 = 4;

pub(crate) struct Block {
	pub(super) ty: u8,
	pub(super) last: bool,
	pub(crate) content: Vec<u8>,
	pub(super) start: u64,
	pub(super) end: u64,
}

impl Block {
	pub(super) const BLOCK_HEADER_SIZE: usize = 4;
	pub(super) const MAX_CONTENT_SIZE: u32 = 16_777_215;

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

		let end = start + u64::from(size) + BLOCK_HEADER_SIZE;

		Ok(Self {
			ty,
			last,
			content,
			start,
			end,
		})
	}

	pub fn len(&self) -> u32 {
		(Self::BLOCK_HEADER_SIZE as u32) + self.content.len() as u32
	}

	pub(super) fn new_padding(size: usize) -> Result<Self> {
		let block_size = core::cmp::min(size, Self::MAX_CONTENT_SIZE as usize);
		let content = try_vec![0; block_size];
		Ok(Self {
			ty: BLOCK_ID_PADDING,
			last: false,
			content,
			start: 0,
			end: 0,
		})
	}

	pub(super) fn new_picture(picture: &Picture, info: PictureInformation) -> Result<Self> {
		let picture_data = picture.as_flac_bytes(info, false);
		if picture_data.len() > Self::MAX_CONTENT_SIZE as usize {
			err!(TooMuchData);
		}

		Ok(Self {
			ty: BLOCK_ID_PICTURE,
			last: false,
			content: picture_data,
			start: 0,
			end: 0,
		})
	}

	pub(super) fn new_comments<'a>(
		vendor: &str,
		items: &mut impl Iterator<Item = (&'a str, &'a str)>,
	) -> Result<Self> {
		let mut comments = Cursor::new(Vec::new());

		comments.write_u32::<LittleEndian>(vendor.len() as u32)?;
		comments.write_all(vendor.as_bytes())?;

		let item_count_pos = comments.stream_position()?;
		let mut count = 0;

		comments.write_u32::<LittleEndian>(count)?;

		crate::ogg::write::create_comments(&mut comments, &mut count, items)?;

		if comments.get_ref().len() > Block::MAX_CONTENT_SIZE as usize {
			err!(TooMuchData);
		}

		comments.seek(SeekFrom::Start(item_count_pos))?;
		comments.write_u32::<LittleEndian>(count)?;

		Ok(Self {
			ty: BLOCK_ID_VORBIS_COMMENTS,
			last: false,
			content: comments.into_inner(),
			start: 0,
			end: 0,
		})
	}

	pub(super) fn write_to<W>(&self, writer: &mut W) -> Result<usize>
	where
		W: Write,
	{
		let block_content_size =
			core::cmp::min(self.content.len(), Self::MAX_CONTENT_SIZE as usize);

		writer.write_u8((self.ty & 0x7F) | u8::from(self.last) << 7)?;
		writer.write_u24::<BigEndian>(block_content_size as u32)?;
		writer.write_all(&self.content)?;

		Ok(Self::BLOCK_HEADER_SIZE + self.content.len())
	}
}
