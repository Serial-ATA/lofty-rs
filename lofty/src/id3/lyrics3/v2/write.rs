use super::constants::{
	BASIC_TEXT_FIELD_MAX_SIZE, FIELD_AUTHOR, FIELD_EXTENDED_ALBUM_NAME, FIELD_EXTENDED_ARTIST_NAME,
	FIELD_EXTENDED_TRACK_TITLE, FIELD_IMAGE, FIELD_INFO, FIELD_LYRICS, FIELD_MAX_SIZE,
	LYRICS3V2_FOOTER, LYRICS3V2_HEADER, PATH_MAX, TAG_MAX_SIZE,
};
use crate::config::WriteOptions;
use crate::error::{FileEncodingError, TagEncodingError, TagParseError, TooMuchDataError};
use crate::id3::lyrics3::v2::Lyrics3v2TagRef;
use crate::id3::lyrics3::v2::errors::Lyrics3v2EncodingError;
use crate::id3::lyrics3::write_latin1_string;
use crate::id3::{ID3FindResults, find_id3v1, find_lyrics3};
use crate::io::VerifiedFile;
use crate::macros::try_vec;
use crate::util::io::FileLike;

use std::io::{Cursor, Seek, SeekFrom, Write};

#[allow(clippy::shadow_unrelated, clippy::collapsible_if)]
pub(crate) fn write_lyrics3v2<F>(
	file: VerifiedFile<'_, F>,
	tag: &Lyrics3v2TagRef<'_>,
	write_options: WriteOptions,
) -> Result<(), FileEncodingError>
where
	F: FileLike,
{
	let mut file = file.into_inner();

	let Some(ID3FindResults {
		header: _,
		content: _,
		range: mut tag_range,
	}) = find_id3v1(&mut file, false, write_options.parse_options.parsing_mode)
		.map_err(TagParseError::from)?
	else {
		return Err(TagEncodingError::from(Lyrics3v2EncodingError::message(
			"the destination file must contain an ID3v1 tag",
		))
		.into());
	};

	// Write immediately before the ID3v1 tag
	tag_range = tag_range.start..tag_range.start;

	if let Some(ID3FindResults {
		header: _,
		content: _,
		range,
	}) = find_lyrics3(&mut file, false, write_options.parse_options)?
	{
		// Replace the existing tag
		tag_range = range;
	}

	let tag = encode(tag, write_options).map_err(TagEncodingError::from)?;
	file.splice(tag_range, &tag)?;

	Ok(())
}

pub(super) fn encode(
	tag: &Lyrics3v2TagRef<'_>,
	write_options: WriteOptions,
) -> Result<Vec<u8>, Lyrics3v2EncodingError> {
	fn write_field_header<W>(
		writer: &mut W,
		id: [u8; 3],
		len: u32,
	) -> Result<(), Lyrics3v2EncodingError>
	where
		W: Write,
	{
		writer.write_all(format!("{}{:05}", id.escape_ascii(), len).as_bytes())?;
		Ok(())
	}

	fn write_field_text<W>(
		context: &str,
		writer: &mut W,
		max_size: u32,
		content: &str,
		write_options: WriteOptions,
	) -> Result<(), Lyrics3v2EncodingError>
	where
		W: Write,
	{
		write_latin1_string(
			context,
			writer,
			content,
			max_size as usize,
			write_options,
			|| Lyrics3v2EncodingError::message(format!("{context} contains invalid characters")),
		)
	}

	fn write_field<W>(
		context: &str,
		writer: &mut W,
		id: [u8; 3],
		max_size: u32,
		content: Option<&str>,
		write_options: WriteOptions,
	) -> Result<(), Lyrics3v2EncodingError>
	where
		W: Write,
	{
		let Some(content) = content else {
			return Ok(());
		};

		let Ok(content_len) = u32::try_from(content.chars().count()) else {
			return Err(TooMuchDataError.into());
		};

		write_field_header(writer, id, content_len.min(max_size))?;
		write_field_text(context, writer, max_size, content, write_options)?;

		Ok(())
	}

	if tag.is_empty() {
		return Ok(Vec::new()); // Stripping
	}

	let tag_content_len = LYRICS3V2_HEADER.len() + tag.len();
	if tag_content_len > TAG_MAX_SIZE as usize {
		return Err(TooMuchDataError.into());
	}

	let mut tag_bytes = Cursor::new(try_vec![0; tag_content_len + 6 + LYRICS3V2_FOOTER.len()]?);

	tag_bytes.write_all(LYRICS3V2_HEADER.as_bytes())?;

	// Indications
	let mut indications = [b'0'; 3];
	tag_bytes.write_all(b"IND00003")?;

	if tag.lyrics.is_some() {
		// Lyrics present
		indications[0] = b'1';

		if tag.flags.synchronized_lyrics {
			indications[1] = b'1';
		}
	}

	if tag.flags.random_selection {
		indications[2] = b'1';
	}

	tag_bytes.write_all(&indications)?;

	write_field(
		"lyrics field",
		&mut tag_bytes,
		FIELD_LYRICS,
		FIELD_MAX_SIZE,
		tag.lyrics,
		write_options,
	)?;
	write_field(
		"info field",
		&mut tag_bytes,
		FIELD_INFO,
		FIELD_MAX_SIZE,
		tag.info,
		write_options,
	)?;
	write_field(
		"author field",
		&mut tag_bytes,
		FIELD_AUTHOR,
		BASIC_TEXT_FIELD_MAX_SIZE,
		tag.author,
		write_options,
	)?;
	write_field(
		"album name",
		&mut tag_bytes,
		FIELD_EXTENDED_ALBUM_NAME,
		BASIC_TEXT_FIELD_MAX_SIZE,
		tag.extended_album_name,
		write_options,
	)?;
	write_field(
		"artist name",
		&mut tag_bytes,
		FIELD_EXTENDED_ARTIST_NAME,
		BASIC_TEXT_FIELD_MAX_SIZE,
		tag.extended_artist_name,
		write_options,
	)?;
	write_field(
		"track title",
		&mut tag_bytes,
		FIELD_EXTENDED_TRACK_TITLE,
		BASIC_TEXT_FIELD_MAX_SIZE,
		tag.extended_track_title,
		write_options,
	)?;

	if !tag.images.is_empty() {
		let total_images = tag.images.len();
		write_field_header(&mut tag_bytes, FIELD_IMAGE, 0)?;

		let start_pos = tag_bytes.stream_position()?;
		for (index, image) in tag.images.iter().enumerate() {
			write_field_text(
				"image filename",
				&mut tag_bytes,
				PATH_MAX,
				image.filename,
				write_options,
			)?;

			tag_bytes.write_all(b"||")?;
			if let Some(description) = image.description {
				write_field_text(
					"image description",
					&mut tag_bytes,
					BASIC_TEXT_FIELD_MAX_SIZE,
					description,
					write_options,
				)?;
			}

			tag_bytes.write_all(b"||")?;
			if let Some(timestamp) = image.timestamp {
				write!(
					&mut tag_bytes,
					"[{:02}:{:02}]",
					std::cmp::min(timestamp.minute.unwrap_or(0), 59),
					std::cmp::min(timestamp.second.unwrap_or(0), 59)
				)?;
			}

			if index != total_images - 1 {
				tag_bytes.write_all(b"\r\n")?;
			}
		}
		let end_pos = tag_bytes.stream_position()?;

		let field_size = end_pos - start_pos;
		if field_size > u64::from(FIELD_MAX_SIZE) {
			return Err(TooMuchDataError.into());
		}

		tag_bytes.seek(SeekFrom::Start(start_pos - 5))?;
		tag_bytes.write_all(format!("{field_size:05}").as_bytes())?;
		tag_bytes.seek(SeekFrom::Start(end_pos))?;
	}

	assert_eq!(
		tag_bytes.stream_position()?,
		tag_content_len as u64,
		"sanity"
	);

	tag_bytes.write_all(format!("{tag_content_len:06}").as_bytes())?;
	tag_bytes.write_all(LYRICS3V2_FOOTER.as_bytes())?;

	Ok(tag_bytes.into_inner())
}
