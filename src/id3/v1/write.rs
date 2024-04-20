use super::tag::Id3v1TagRef;
use crate::config::WriteOptions;
use crate::error::{LoftyError, Result};
use crate::id3::{find_id3v1, ID3FindResults};
use crate::macros::err;
use crate::probe::Probe;
use crate::util::io::{FileLike, Length, Truncate};

use std::io::{Cursor, Seek, Write};

use byteorder::WriteBytesExt;

#[allow(clippy::shadow_unrelated)]
pub(crate) fn write_id3v1<F>(
	file: &mut F,
	tag: &Id3v1TagRef<'_>,
	_write_options: WriteOptions,
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
	let ID3FindResults(header, _) = find_id3v1(file, false)?;

	if tag.is_empty() && header.is_some() {
		// An ID3v1 tag occupies the last 128 bytes of the file, so we can just
		// shrink it down.
		let new_length = file.len()?.saturating_sub(128);
		file.truncate(new_length)?;

		return Ok(());
	}

	let tag = encode(tag)?;

	file.write_all(&tag)?;

	Ok(())
}

pub(super) fn encode(tag: &Id3v1TagRef<'_>) -> std::io::Result<Vec<u8>> {
	fn resize_string(value: Option<&str>, size: usize) -> std::io::Result<Vec<u8>> {
		let mut cursor = Cursor::new(vec![0; size]);
		cursor.rewind()?;

		if let Some(val) = value {
			if val.len() > size {
				cursor.write_all(val.split_at(size).0.as_bytes())?;
			} else {
				cursor.write_all(val.as_bytes())?;
			}
		}

		Ok(cursor.into_inner())
	}

	let mut writer = Vec::with_capacity(128);

	writer.write_all(&[b'T', b'A', b'G'])?;

	let title = resize_string(tag.title, 30)?;
	writer.write_all(&title)?;

	let artist = resize_string(tag.artist, 30)?;
	writer.write_all(&artist)?;

	let album = resize_string(tag.album, 30)?;
	writer.write_all(&album)?;

	let year = resize_string(tag.year, 4)?;
	writer.write_all(&year)?;

	let comment = resize_string(tag.comment, 28)?;
	writer.write_all(&comment)?;

	writer.write_u8(0)?;

	writer.write_u8(tag.track_number.unwrap_or(0))?;
	writer.write_u8(tag.genre.unwrap_or(255))?;

	Ok(writer)
}
