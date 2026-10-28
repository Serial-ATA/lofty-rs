use super::constants::{LYRICS3_FOOTER, LYRICS3_HEADER, MAX_LYRICS_LEN};
use crate::config::WriteOptions;
use crate::error::{FileEncodingError, TagEncodingError, TagParseError};
use crate::id3::lyrics3::v1::Lyrics3TagRef;
use crate::id3::lyrics3::v1::errors::Lyrics3EncodingError;
use crate::id3::{ID3FindResults, find_id3v1, find_lyrics3};
use crate::io::VerifiedFile;
use crate::macros::try_vec;
use crate::util::io::FileLike;
use crate::util::text::latin1_encode;

use std::io::{Cursor, Write};

use byteorder::WriteBytesExt;

#[allow(clippy::shadow_unrelated, clippy::collapsible_if)]
pub(crate) fn write_lyrics3<F>(
	file: VerifiedFile<'_, F>,
	tag: &Lyrics3TagRef<'_>,
	write_options: WriteOptions,
) -> Result<(), FileEncodingError>
where
	F: FileLike,
{
	let mut file = file.into_inner();

	let Some(ID3FindResults {
		header: _,
		content: _,
		// Write immediately before the ID3v1 tag
		range: mut tag_range,
	}) = find_id3v1(&mut file, false, write_options.parse_options.parsing_mode)
		.map_err(TagParseError::from)?
	else {
		return Err(TagEncodingError::from(Lyrics3EncodingError::message(
			"the destination file must contain an ID3v1 tag",
		))
		.into());
	};

	if let Some(ID3FindResults {
		header: _,
		content: _,
		range,
	}) = find_lyrics3(&mut file, false)?
	{
		// Replace the existing tag
		tag_range = range;
	}

	let tag = encode(tag, write_options).map_err(TagEncodingError::from)?;
	file.splice(tag_range, &tag)?;

	Ok(())
}

pub(super) fn encode(
	tag: &Lyrics3TagRef<'_>,
	write_options: WriteOptions,
) -> Result<Vec<u8>, Lyrics3EncodingError> {
	let Some(lyrics) = tag.lyrics else {
		return Ok(Vec::new()); // Stripping
	};

	if lyrics.contains(LYRICS3_HEADER) || lyrics.contains(LYRICS3_FOOTER) {
		return Err(Lyrics3EncodingError::message(
			"lyrics text contains tag markers",
		));
	}

	let mut tag =
		Cursor::new(try_vec![0; LYRICS3_HEADER.len() + lyrics.len() + LYRICS3_FOOTER.len()]?);

	tag.write_all(LYRICS3_HEADER.as_bytes())?;

	for b in latin1_encode(lyrics, write_options.lossy_text_encoding).take(MAX_LYRICS_LEN) {
		let mut b = b?;
		if b == 255 {
			if write_options.lossy_text_encoding {
				b = b'?';
			} else {
				return Err(Lyrics3EncodingError::message(
					"lyrics text contains invalid characters",
				));
			}
		}

		tag.write_u8(b)?;
	}

	tag.write_all(LYRICS3_FOOTER.as_bytes())?;

	Ok(tag.into_inner())
}
