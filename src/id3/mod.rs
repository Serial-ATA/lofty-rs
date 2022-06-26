//! ID3 specific items
//!
//! ID3 does things differently than other tags, making working with them a little more effort than other formats.
//! Check the other modules for important notes and/or warnings.

pub mod v1;
pub mod v2;

use crate::error::{ErrorKind, LoftyError, Result};
use crate::macros::try_vec;
use v2::{read_id3v2_header, ID3v2Header};

use std::io::{Read, Seek, SeekFrom};
use std::ops::Neg;

pub(crate) struct ID3FindResults<Header, Content>(pub Option<Header>, pub Content);

pub(crate) fn find_lyrics3v2<R>(data: &mut R) -> Result<ID3FindResults<(), u32>>
where
	R: Read + Seek,
{
	let mut header = None;
	let mut size = 0_u32;

	data.seek(SeekFrom::Current(-15))?;

	let mut lyrics3v2 = [0; 15];
	data.read_exact(&mut lyrics3v2)?;

	if &lyrics3v2[7..] == b"LYRICS200" {
		header = Some(());

		let lyrics_size = std::str::from_utf8(&lyrics3v2[..7])?;
		let lyrics_size = lyrics_size.parse::<u32>().map_err(|_| {
			LoftyError::new(ErrorKind::TextDecode(
				"Lyrics3v2 tag has an invalid size string",
			))
		})?;

		size += lyrics_size;

		data.seek(SeekFrom::Current(i64::from(lyrics_size + 15).neg()))?;
	}

	Ok(ID3FindResults(header, size))
}

#[cfg(feature = "id3v1")]
pub(crate) type FindID3v1Content = Option<v1::tag::Id3v1Tag>;
#[cfg(not(feature = "id3v1"))]
pub(crate) type FindID3v1Content = Option<()>;

#[allow(unused_variables)]
pub(crate) fn find_id3v1<R>(
	data: &mut R,
	read: bool,
) -> Result<ID3FindResults<(), FindID3v1Content>>
where
	R: Read + Seek,
{
	let mut id3v1 = None;
	let mut header = None;

	data.seek(SeekFrom::End(-128))?;

	let mut id3v1_header = [0; 3];
	data.read_exact(&mut id3v1_header)?;

	data.seek(SeekFrom::Current(-3))?;

	if &id3v1_header == b"TAG" {
		header = Some(());

		#[cfg(feature = "id3v1")]
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

	Ok(ID3FindResults(header, id3v1))
}

pub(crate) fn find_id3v2<R>(
	data: &mut R,
	read: bool,
) -> Result<ID3FindResults<ID3v2Header, Option<Vec<u8>>>>
where
	R: Read + Seek,
{
	let mut header = None;
	let mut id3v2 = None;

	if let Ok(id3v2_header) = read_id3v2_header(data) {
		if read {
			let mut tag = try_vec![0; id3v2_header.size as usize];
			data.read_exact(&mut tag)?;

			id3v2 = Some(tag)
		} else {
			data.seek(SeekFrom::Current(i64::from(id3v2_header.size)))?;
		}

		if id3v2_header.flags.footer {
			data.seek(SeekFrom::Current(10))?;
		}

		header = Some(id3v2_header);
	} else {
		data.seek(SeekFrom::Current(-10))?;
	}

	Ok(ID3FindResults(header, id3v2))
}
