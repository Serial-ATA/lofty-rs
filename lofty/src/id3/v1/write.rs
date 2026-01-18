use super::constants::ID3V1_TAG_MARKER;
use super::tag::Id3v1TagRef;
use crate::config::{ParseOptions, WriteOptions};
use crate::error::{LoftyError, Result};
use crate::id3::{ID3FindResults, find_id3v1};
use crate::macros::err;
use crate::probe::Probe;
use crate::util::io::{FileLike, Length, Truncate};
use crate::util::text::latin1_encode;

use std::io::{Cursor, Seek, Write};

use byteorder::WriteBytesExt;

#[allow(clippy::shadow_unrelated)]
pub(crate) fn write_id3v1<F>(
	file: &mut F,
	tag: &Id3v1TagRef<'_>,
	write_options: WriteOptions,
) -> Result<()>
where
	F: FileLike,
	LoftyError: From<<F as Truncate>::Error>,
	LoftyError: From<<F as Length>::Error>,
{
	let probe = Probe::new(file).guess_file_type()?;

	match probe.file_type() {
		Some(ft) if super::Id3v1Tag::SUPPORTED_FORMATS.contains(&ft) => {},
		_ => err!(UnsupportedTag),
	}

	let file = probe.into_inner();

	// This will seek us to the writing position
	// TODO: Forcing the use of ParseOptions::default()
	let parse_options = ParseOptions::default();
	let ID3FindResults(header, _) = find_id3v1(file, false, parse_options.parsing_mode)?;

	if tag.is_empty() && header.is_some() {
		// An ID3v1 tag occupies the last 128 bytes of the file, so we can just
		// shrink it down.
		let new_length = file.len()?.saturating_sub(128);
		file.truncate(new_length)?;

		return Ok(());
	}

	let tag = encode(tag, write_options)?;

	file.write_all(&tag)?;

	Ok(())
}

pub(super) fn encode(tag: &Id3v1TagRef<'_>, write_options: WriteOptions) -> Result<Vec<u8>> {
	fn resize_string(
		value: Option<&str>,
		size: usize,
		write_options: WriteOptions,
	) -> Result<Vec<u8>> {
		let mut cursor = Cursor::new(vec![0; size]);
		cursor.rewind()?;

		if let Some(val) = value {
			for b in latin1_encode(val, write_options.lossy_text_encoding).take(size) {
				cursor.write_u8(b?)?;
			}
		}

		Ok(cursor.into_inner())
	}

	let mut writer = Vec::with_capacity(128);

	writer.write_all(&ID3V1_TAG_MARKER)?;

	let title = resize_string(tag.title, 30, write_options)?;
	writer.write_all(&title)?;

	let artist = resize_string(tag.artist, 30, write_options)?;
	writer.write_all(&artist)?;

	let album = resize_string(tag.album, 30, write_options)?;
	writer.write_all(&album)?;

	let mut year = [0; 4];
	if let Some(year_num) = tag.year {
		let mut year_num = std::cmp::min(year_num, 9999);

		let mut idx = 3;
		loop {
			year[idx] = b'0' + (year_num % 10) as u8;
			year_num /= 10;

			if idx == 0 {
				break;
			}

			idx -= 1;
		}
	}

	writer.write_all(&year)?;

	let comment = resize_string(tag.comment, 28, write_options)?;
	writer.write_all(&comment)?;

	writer.write_u8(0)?;

	writer.write_u8(tag.track_number.unwrap_or(0))?;
	writer.write_u8(tag.genre.unwrap_or(255))?;

	Ok(writer)
}
