use crate::error::Result;
use crate::logic::id3::unsynch_u32;

use std::io::{Read, Seek, SeekFrom};

use byteorder::{BigEndian, ByteOrder};

pub(crate) mod frame;
pub(crate) mod items;
pub(crate) mod read;
pub(crate) mod util;

#[derive(PartialEq, Debug, Clone, Copy)]
/// The ID3v2 version
pub enum Id3v2Version {
	/// ID3v2.2
	V2,
	/// ID3v2.3
	V3,
	/// ID3v2.4
	V4,
}

pub(crate) fn find_id3v2<R>(data: &mut R, read: bool) -> Result<Option<Vec<u8>>>
where
	R: Read + Seek,
{
	let mut id3v2 = None;

	let mut id3_header = [0; 10];
	data.read_exact(&mut id3_header)?;

	if &id3_header[..3] == b"ID3" {
		let size = unsynch_u32(BigEndian::read_u32(&id3_header[6..]));

		if read {
			data.seek(SeekFrom::Current(-10))?;

			let mut tag = vec![0; (size + 10) as usize];
			data.read_exact(&mut tag)?;

			id3v2 = Some(tag)
		} else {
			data.seek(SeekFrom::Current(i64::from(size)))?;
		}
	}

	Ok(id3v2)
}
