//! ID3 specific items
//!
//! ID3 does things differently than other tags, making working with them a little more effort than other formats.
//! Check the other modules for important notes and/or warnings.

pub mod v1;
pub mod v2;

use crate::config::ParsingMode;
use crate::error::{ErrorKind, LoftyError, Result};
use crate::macros::try_vec;
use crate::util::text::utf8_decode_str;
use v1::constants::ID3V1_TAG_MARKER;
use v2::header::Id3v2Header;

use std::io::{Read, Seek, SeekFrom};
use std::ops::Neg;

pub(crate) struct ID3FindResults<Header, Content>(pub Option<Header>, pub Content);

pub(crate) fn find_lyrics3v2<R>(data: &mut R) -> Result<ID3FindResults<(), u32>>
where
	R: Read + Seek,
{
	log::debug!("Searching for a Lyrics3v2 tag");

	let mut header = None;
	let mut size = 0_u32;

	data.seek(SeekFrom::Current(-15))?;

	let mut lyrics3v2 = [0; 15];
	data.read_exact(&mut lyrics3v2)?;

	if &lyrics3v2[7..] == b"LYRICS200" {
		log::warn!("Encountered a Lyrics3v2 tag. This is an outdated format, and will be skipped.");

		header = Some(());

		let lyrics_size = utf8_decode_str(&lyrics3v2[..7])?;
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

#[allow(unused_variables)]
pub(crate) fn find_id3v1<R>(
	data: &mut R,
	read: bool,
	parse_mode: ParsingMode,
) -> Result<ID3FindResults<(), Option<v1::tag::Id3v1Tag>>>
where
	R: Read + Seek,
{
	log::debug!("Searching for an ID3v1 tag");

	let mut id3v1 = None;
	let mut header = None;

	// Reader is too small to contain an ID3v2 tag
	if data.seek(SeekFrom::End(-128)).is_err() {
		data.seek(SeekFrom::End(0))?;
		return Ok(ID3FindResults(header, id3v1));
	}

	let mut id3v1_header = [0; 3];
	data.read_exact(&mut id3v1_header)?;

	data.seek(SeekFrom::Current(-3))?;

	// No ID3v1 tag found
	if id3v1_header != ID3V1_TAG_MARKER {
		data.seek(SeekFrom::End(0))?;
		return Ok(ID3FindResults(header, id3v1));
	}

	log::debug!("Found an ID3v1 tag, parsing");

	header = Some(());

	if read {
		let mut id3v1_tag = [0; 128];
		data.read_exact(&mut id3v1_tag)?;

		data.seek(SeekFrom::End(-128))?;

		id3v1 = Some(v1::tag::Id3v1Tag::parse(id3v1_tag, parse_mode)?)
	}

	Ok(ID3FindResults(header, id3v1))
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct FindId3v2Config {
	pub(crate) read: bool,
	pub(crate) allowed_junk_window: Option<u64>,
}

impl FindId3v2Config {
	pub(crate) const NO_READ_TAG: Self = Self {
		read: false,
		allowed_junk_window: None,
	};

	pub(crate) const READ_TAG: Self = Self {
		read: true,
		allowed_junk_window: None,
	};
}

pub(crate) fn find_id3v2<R>(
	data: &mut R,
	config: FindId3v2Config,
) -> Result<ID3FindResults<Id3v2Header, Option<Vec<u8>>>>
where
	R: Read + Seek,
{
	log::debug!(
		"Searching for an ID3v2 tag at offset: {}",
		data.stream_position()?
	);

	let mut header = None;
	let mut id3v2 = None;

	if let Some(junk_window) = config.allowed_junk_window {
		let mut id3v2_search_window = data.by_ref().take(junk_window);

		let Some(id3v2_offset) = find_id3v2_in_junk(&mut id3v2_search_window)? else {
			return Ok(ID3FindResults(None, None));
		};

		log::warn!(
			"Found an ID3v2 tag preceded by junk data, offset: {}",
			id3v2_offset
		);

		data.seek(SeekFrom::Current(-3))?;
	}

	if let Ok(id3v2_header) = Id3v2Header::parse(data) {
		log::debug!("Found an ID3v2 tag, parsing");

		if config.read {
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

/// Searches for an ID3v2 tag in (potential) junk data between the start
/// of the file and the first frame
fn find_id3v2_in_junk<R>(reader: &mut R) -> Result<Option<u64>>
where
	R: Read,
{
	let bytes = reader.bytes();

	let mut id3v2_header = [0; 3];

	for (index, byte) in bytes.enumerate() {
		id3v2_header[0] = id3v2_header[1];
		id3v2_header[1] = id3v2_header[2];
		id3v2_header[2] = byte?;
		if id3v2_header == *b"ID3" {
			return Ok(Some((index - 2) as u64));
		}
	}

	Ok(None)
}
