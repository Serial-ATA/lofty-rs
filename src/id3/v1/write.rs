use super::tag::Id3v1TagRef;
use crate::error::Result;
use crate::file::FileType;
use crate::id3::{find_id3v1, ID3FindResults};
use crate::macros::err;
use crate::probe::Probe;

use std::fs::File;
use std::io::{Cursor, Read, Seek, Write};

use byteorder::WriteBytesExt;

#[allow(clippy::shadow_unrelated)]
pub(crate) fn write_id3v1(writer: &mut File, tag: &Id3v1TagRef<'_>) -> Result<()> {
	let probe = Probe::new(writer).guess_file_type()?;

	match probe.file_type() {
		Some(FileType::APE | FileType::MPEG | FileType::WavPack) => {},
		_ => err!(UnsupportedTag),
	}

	let writer = probe.into_inner();

	// This will seek us to the writing position
	let ID3FindResults(header, _) = find_id3v1(writer, false)?;

	if tag.is_empty() && header.is_some() {
		writer.rewind()?;

		let mut file_bytes = Vec::new();
		writer.read_to_end(&mut file_bytes)?;

		writer.rewind()?;
		writer.set_len(0)?;
		writer.write_all(&file_bytes[..file_bytes.len() - 128])?;

		return Ok(());
	}

	let tag = encode(tag)?;

	writer.write_all(&tag)?;

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
