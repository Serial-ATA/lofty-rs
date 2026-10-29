use super::constants::ID3V1_TAG_MARKER;
use super::tag::Id3v1TagRef;
use crate::config::WriteOptions;
use crate::error::{FileEncodingError, TagEncodingError, TagParseError, TooMuchDataError};
use crate::id3::v1::error::Id3v1EncodingError;
use crate::id3::{ID3FindResults, find_id3v1, find_lyrics3};
use crate::io::VerifiedFile;
use crate::util::io::FileLike;
use crate::util::text::latin1_encode;

use std::io::{Cursor, Seek, Write};

use byteorder::WriteBytesExt;

#[allow(clippy::shadow_unrelated, clippy::collapsible_if)]
pub(crate) fn write_id3v1<F>(
	file: VerifiedFile<'_, F>,
	tag: &Id3v1TagRef<'_>,
	write_options: WriteOptions,
) -> Result<(), FileEncodingError>
where
	F: FileLike,
{
	let mut file = file.into_inner();

	// This will seek us to the writing position
	let existing_tag_range = find_id3v1(&mut file, false, write_options.parse_options.parsing_mode)
		.map_err(TagParseError::from)?
		.map(
			|ID3FindResults {
			     header: _,
			     content: _,
			     range,
			 }| range,
		);

	if tag.is_empty() {
		let Some(mut existing_tag_range) = existing_tag_range else {
			log::debug!("No tag to overwrite, nothing to do");
			return Ok(());
		};

		if let Some(ID3FindResults {
			header: _,
			content: _,
			range,
		}) = find_lyrics3(&mut file, false, write_options.parse_options)?
		{
			log::warn!("Stripping Lyrics3 tag");

			// Extend back to remove the Lyrics3 tag too.
			//
			// Lyrics3 is an extension of ID3v1, and the spec mandates that it can't be standalone.
			existing_tag_range.start = range.start;
		}

		file.splice(existing_tag_range, &[])?;
		return Ok(());
	}

	let tag = encode(tag, write_options).map_err(TagEncodingError::from)?;
	file.write_all(&tag)?;

	Ok(())
}

pub(super) fn encode(
	tag: &Id3v1TagRef<'_>,
	write_options: WriteOptions,
) -> Result<Vec<u8>, Id3v1EncodingError> {
	fn resize_string(
		value: Option<&str>,
		size: usize,
		write_options: WriteOptions,
	) -> Result<Vec<u8>, Id3v1EncodingError> {
		let mut cursor = Cursor::new(vec![0; size]);
		cursor.rewind()?;

		if let Some(val) = value {
			if val.chars().count() > size && !write_options.implicit_truncation {
				return Err(TooMuchDataError.into());
			}

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
