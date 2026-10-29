//! ID3 specific items
//!
//! This covers both [`v1`] (ID3v1) and [`v2`] (ID3v2) tag formats.
//!
//! ID3 does things differently than other tags, making them a little more cumbersome to work with.
//! Check the other modules for important notes and warnings.

pub mod lyrics3;
pub mod v1;
pub mod v2;

use crate::config::{ParseOptions, ParsingMode};
use crate::error::{FakeTagError, TagParseError};
use crate::id3::lyrics3::v1::constants::{LYRICS3_FOOTER, LYRICS3_HEADER};
use crate::id3::lyrics3::v1::errors::Lyrics3ParseError;
use crate::id3::lyrics3::v2::constants::{LYRICS3V2_FOOTER, LYRICS3V2_HEADER};
use crate::id3::lyrics3::v2::errors::Lyrics3v2ParseError;
use crate::id3::v1::error::Id3v1ParseError;
use crate::id3::v2::error::Id3v2ParseError;
use crate::io::{ReadFindExt, RevSearchEnd};
use crate::macros::try_vec;
use v1::constants::ID3V1_TAG_MARKER;
use v2::header::Id3v2Header;

use std::io::{Read, Seek, SeekFrom};
use std::ops::Range;

#[derive(Debug)]
pub(crate) struct ID3FindResults<Header, Content> {
	pub header: Header,
	pub content: Content,
	/// The byte range of the tag
	pub range: Range<u64>,
}

#[derive(Debug)]
pub(crate) enum Lyrics3Tag {
	V1(lyrics3::v1::Lyrics3Tag),
	V2(lyrics3::v2::Lyrics3v2Tag),
}

/// Attempt to find a Lyrics3 (v1 or v2) tag at the current position
///
/// `read` determines if the tag is parsed, if one exists.
///
/// If found, the `reader` will be positioned at the start of the tag. Otherwise, it will be at the
/// original position.
pub(crate) fn find_lyrics3<R>(
	reader: &mut R,
	read: bool,
	parse_options: ParseOptions,
) -> Result<Option<ID3FindResults<(), Option<Lyrics3Tag>>>, TagParseError>
where
	R: Read + Seek,
{
	fn handle_v1<R>(
		reader: &mut R,
		read: bool,
		tag_end_pos: u64,
	) -> Result<(Range<u64>, Option<Lyrics3Tag>), Lyrics3ParseError>
	where
		R: Read + Seek,
	{
		const MAX_TAG_SIZE: u64 = LYRICS3_HEADER.len() as u64
			+ lyrics3::v1::constants::MAX_LYRICS_LEN as u64
			+ LYRICS3_FOOTER.len() as u64;

		if !reader
			.rfind(LYRICS3_HEADER.as_bytes())
			.end_pos(RevSearchEnd::Pos(tag_end_pos.saturating_sub(MAX_TAG_SIZE)))
			.search()?
		{
			return Err(FakeTagError.into());
		}

		let tag_start = reader.stream_position()?;
		let range = tag_start..tag_end_pos;

		let mut tag = None;
		if read {
			let mut content = try_vec![0; (range.end - range.start) as usize]?;
			reader.read_exact(&mut content)?;

			tag = Some(lyrics3::v1::Lyrics3Tag::parse(&content)?);
			reader.seek(SeekFrom::Start(range.start))?;
		}

		Ok((range, tag.map(Lyrics3Tag::V1)))
	}

	fn handle_v2<R>(
		reader: &mut R,
		read: bool,
		tag_end_pos: u64,
		parse_options: ParseOptions,
	) -> Result<(Range<u64>, Option<Lyrics3Tag>), Lyrics3v2ParseError>
	where
		R: Read + Seek,
	{
		const MAX_TAG_SIZE: u64 = lyrics3::v2::constants::TAG_MAX_SIZE as u64 + 6 /* tag size */ + LYRICS3_FOOTER.len() as u64;

		// Don't trust the encoded tag size at this point, just search for the header
		if !reader
			.rfind(LYRICS3V2_HEADER.as_bytes())
			.end_pos(RevSearchEnd::Pos(tag_end_pos.saturating_sub(MAX_TAG_SIZE)))
			.search()?
		{
			return Err(FakeTagError.into());
		}

		let tag_start = reader.stream_position()?;
		let range = tag_start..tag_end_pos;

		let mut tag = None;
		if read {
			let mut content = try_vec![0; (range.end - range.start) as usize]?;
			reader.read_exact(&mut content)?;

			tag = Some(lyrics3::v2::Lyrics3v2Tag::parse(&content, parse_options)?);
			reader.seek(SeekFrom::Start(range.start))?;
		}

		Ok((range, tag.map(Lyrics3Tag::V2)))
	}

	log::debug!("Searching for a Lyrics3 tag");

	reader
		.seek(SeekFrom::Current(-(LYRICS3_FOOTER.len() as i64)))
		.map_err(|e| TagParseError::from_unknown_ty(e.into()))?;

	let mut lyrics3_footer = [0; LYRICS3_FOOTER.len()];
	reader
		.read_exact(&mut lyrics3_footer)
		.map_err(|e| TagParseError::from_unknown_ty(e.into()))?;

	let tag_end = reader
		.stream_position()
		.map_err(|e| TagParseError::from_unknown_ty(e.into()))?;

	let range;
	let tag;
	if lyrics3_footer == LYRICS3_FOOTER.as_bytes() {
		(range, tag) = handle_v1(reader, read, tag_end)?;
	} else if lyrics3_footer == LYRICS3V2_FOOTER.as_bytes() {
		(range, tag) = handle_v2(reader, read, tag_end, parse_options)?;
	} else {
		return Ok(None);
	}

	Ok(Some(ID3FindResults {
		header: (),
		content: tag,
		range,
	}))
}

/// Attempt to find an ID3v1 tag in the stream
///
/// `read` determines if the tag is parsed, if one exists.
///
/// If found, the `reader` will be positioned at the start of the tag. Otherwise, it will be at the
/// end of the stream.
#[allow(unused_variables)]
pub(crate) fn find_id3v1<R>(
	reader: &mut R,
	read: bool,
	parse_mode: ParsingMode,
) -> Result<Option<ID3FindResults<(), Option<v1::tag::Id3v1Tag>>>, Id3v1ParseError>
where
	R: Read + Seek,
{
	log::debug!("Searching for an ID3v1 tag");

	let mut id3v1 = None;

	// Reader is too small to contain an ID3v2 tag
	if reader.seek(SeekFrom::End(-128)).is_err() {
		reader.seek(SeekFrom::End(0))?;
		return Ok(None);
	}

	let mut id3v1_header = [0; 3];
	reader.read_exact(&mut id3v1_header)?;

	let start = reader.seek(SeekFrom::Current(-3))?;

	// No ID3v1 tag found
	if id3v1_header != ID3V1_TAG_MARKER {
		reader.seek(SeekFrom::End(0))?;
		return Ok(None);
	}

	log::debug!("Found an ID3v1 tag, parsing");

	if read {
		let mut id3v1_tag = [0; 128];
		reader.read_exact(&mut id3v1_tag)?;

		reader.seek(SeekFrom::End(-128))?;

		id3v1 = Some(v1::tag::Id3v1Tag::parse(id3v1_tag, parse_mode)?)
	}

	Ok(Some(ID3FindResults {
		header: (),
		content: id3v1,
		range: start..(start + 128),
	}))
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

/// Attempt to find an ID3v2 tag header from the current position in the stream
///
/// If a tag is found, the position of `reader` will be at the end of the tag. Otherwise,
/// the position will be unchanged.
pub(crate) fn find_id3v2<R>(
	reader: &mut R,
	config: FindId3v2Config,
) -> Result<Option<ID3FindResults<Id3v2Header, Option<Vec<u8>>>>, Id3v2ParseError>
where
	R: Read + Seek,
{
	let start = reader.stream_position()?;
	log::debug!("Searching for an ID3v2 tag at offset: {start}");

	let mut id3v2 = None;

	if let Some(junk_window) = config.allowed_junk_window {
		let mut id3v2_search_window = reader.by_ref().take(junk_window);

		let Some(id3v2_offset) = find_id3v2_in_junk(&mut id3v2_search_window)? else {
			reader.seek(SeekFrom::Start(start))?;
			return Ok(None);
		};

		log::warn!(
			"Found a potential ID3v2 tag preceded by junk data, offset: {}",
			id3v2_offset
		);

		reader.seek(SeekFrom::Current(-3))?;
	}

	if let Ok(id3v2_header) = Id3v2Header::parse(reader) {
		log::debug!("Found an ID3v2 tag, parsing");

		if config.read {
			let mut tag = try_vec![0; id3v2_header.size as usize]?;
			reader.read_exact(&mut tag)?;

			id3v2 = Some(tag)
		} else {
			reader.seek(SeekFrom::Current(i64::from(id3v2_header.size)))?;
		}

		if id3v2_header.flags.footer {
			reader.seek(SeekFrom::Current(10))?;
		}

		let end = reader.stream_position()?;
		Ok(Some(ID3FindResults {
			header: id3v2_header,
			content: id3v2,
			range: start..end,
		}))
	} else {
		reader.seek(SeekFrom::Start(start))?;
		Ok(None)
	}
}

/// Searches for an ID3v2 tag in (potential) junk data between the start
/// of the file and the first frame
fn find_id3v2_in_junk<R>(reader: &mut R) -> Result<Option<u64>, Id3v2ParseError>
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::config::ParseOptions;

	use std::io::Cursor;

	#[test_log::test]
	fn find_id3v2_seek_back() {
		let mut data = [0; 32];
		data[20..23].copy_from_slice(b"ID3");

		let mut cursor = Cursor::new(data);
		cursor.seek(SeekFrom::Start(5)).unwrap();

		let mut config = FindId3v2Config::READ_TAG;
		config.allowed_junk_window = Some(ParseOptions::DEFAULT_MAX_JUNK_BYTES as u64);

		assert!(find_id3v2(&mut cursor, config).unwrap().is_none());
		assert_eq!(
			cursor.position(),
			5,
			"`find_id3v2` should seek back to the original position"
		);
	}
}
