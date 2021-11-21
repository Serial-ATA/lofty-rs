#[cfg(feature = "id3v1")]
pub(crate) mod v1;

#[cfg(feature = "id3v2")]
pub(crate) mod v2;

use crate::{LoftyError, Result};

use std::io::{Read, Seek, SeekFrom};
use std::ops::Neg;

// https://github.com/polyfloyd/rust-id3/blob/e142ec656bf70a8153f6e5b34a37f26df144c3c1/src/stream/unsynch.rs#L18-L20
pub(crate) fn unsynch_u32(n: u32) -> u32 {
	n & 0xFF | (n & 0xFF00) >> 1 | (n & 0xFF_0000) >> 2 | (n & 0xFF00_0000) >> 3
}

// https://github.com/polyfloyd/rust-id3/blob/e142ec656bf70a8153f6e5b34a37f26df144c3c1/src/stream/unsynch.rs#L9-L15
pub(crate) fn synch_u32(n: u32) -> Result<u32> {
	if n > 0x1000_0000 {
		return Err(LoftyError::TooMuchData);
	}

	let mut x: u32 = n & 0x7F | (n & 0xFFFF_FF80) << 1;
	x = x & 0x7FFF | (x & 0xFFFF_8000) << 1;
	x = x & 0x7F_FFFF | (x & 0xFF80_0000) << 1;
	Ok(x)
}

pub(crate) fn find_lyrics3v2<R>(data: &mut R) -> Result<(bool, u32)>
where
	R: Read + Seek,
{
	let mut exists = false;
	let mut size = 0_u32;

	data.seek(SeekFrom::Current(-15))?;

	let mut lyrics3v2 = [0; 15];
	data.read_exact(&mut lyrics3v2)?;

	if &lyrics3v2[7..] == b"LYRICS200" {
		exists = true;

		let lyrics_size = String::from_utf8(lyrics3v2[..7].to_vec())?;
		let lyrics_size = lyrics_size
			.parse::<u32>()
			.map_err(|_| LoftyError::Ape("Lyrics3v2 tag has an invalid size string"))?;

		size += lyrics_size;

		data.seek(SeekFrom::Current(i64::from(lyrics_size + 15).neg()))?;
	}

	Ok((exists, size))
}

#[cfg(feature = "id3v1")]
pub(in crate::logic) fn find_id3v1<R>(
	data: &mut R,
	read: bool,
) -> Result<(bool, Option<v1::tag::Id3v1Tag>)>
where
	R: Read + Seek,
{
	let mut id3v1 = None;
	let mut exists = false;

	data.seek(SeekFrom::End(-128))?;

	let mut id3v1_header = [0; 3];
	data.read_exact(&mut id3v1_header)?;

	data.seek(SeekFrom::Current(-3))?;

	if &id3v1_header == b"TAG" {
		exists = true;

		if read {
			let mut id3v1_tag = [0; 128];
			data.read_exact(&mut id3v1_tag)?;

			data.seek(SeekFrom::End(-128))?;

			id3v1 = Some(v1::read::parse_id3v1(id3v1_tag))
		}
	} else {
		// No ID3v1 tag found
		data.seek(SeekFrom::End(0))?;
	}

	Ok((exists, id3v1))
}

#[cfg(not(feature = "id3v1"))]
pub(in crate::logic) fn find_id3v1<R>(data: &mut R, read: bool) -> Result<(bool, Option<()>)>
where
	R: Read + Seek,
{
	let mut exists = false;

	data.seek(SeekFrom::End(-128))?;

	let mut id3v1_header = [0; 3];
	data.read_exact(&mut id3v1_header)?;

	data.seek(SeekFrom::Current(-3))?;

	if &id3v1_header == b"TAG" {
		exists = true;
	} else {
		// No ID3v1 tag found
		data.seek(SeekFrom::End(0))?;
	}

	Ok((exists, None))
}
