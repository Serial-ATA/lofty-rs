use super::constants::{
	BASIC_TEXT_FIELD_MAX_SIZE, FIELD_AUTHOR, FIELD_EXTENDED_ALBUM_NAME, FIELD_EXTENDED_ARTIST_NAME,
	FIELD_EXTENDED_TRACK_TITLE, FIELD_IMAGE, FIELD_INDICATORS, FIELD_INFO, FIELD_LYRICS,
	FIELD_MAX_SIZE, LYRICS3V2_FOOTER, LYRICS3V2_HEADER,
};
use super::errors::Lyrics3v2ParseError;
use super::{Image, Lyrics3v2Tag};
use crate::config::{ParseOptions, ParsingMode};
use crate::error::{FakeTagError, SizeMismatchError, TooMuchDataError};
use crate::macros::try_vec;
use crate::tag::items::Timestamp;
use crate::util::text::{ascii_number, latin1_decode};

use std::io::Read;

#[derive(Copy, Clone, Debug)]
struct FieldHeader {
	id: [u8; 3],
	size: u32,
}

fn field<R>(reader: &mut R) -> Result<FieldHeader, Lyrics3v2ParseError>
where
	R: Read,
{
	let mut id = [0; 3];
	reader.read_exact(&mut id)?;

	let mut size_str = [0; 5];
	reader.read_exact(&mut size_str)?;

	let (size_bytes, size) = ascii_number(&size_str);
	if size_bytes != 5 {
		return Err(Lyrics3v2ParseError::message("non-numeric field size"));
	}

	Ok(FieldHeader {
		id,
		size: size as u32,
	})
}

fn parse_text<R>(
	header: FieldHeader,
	reader: &mut R,
	field_max: u32,
	parse_mode: ParsingMode,
) -> Result<String, Lyrics3v2ParseError>
where
	R: Read,
{
	if header.size > field_max {
		if parse_mode == ParsingMode::Strict {
			return Err(TooMuchDataError.into());
		}

		log::warn!(
			"Lyrics3v2 field `{}` is too large, truncating",
			header.id.escape_ascii()
		);
	}

	let mut content = try_vec![0; header.size as usize]?;
	reader.read_exact(&mut content)?;
	content.truncate(field_max as usize);

	Ok(latin1_decode(&content))
}

fn parse_timestamp(s: &str) -> Result<Option<Timestamp>, Lyrics3v2ParseError> {
	// TODO: strip_circumfix() (MSRV 1.98)
	let Some(rest) = s.strip_prefix('[') else {
		return Err(Lyrics3v2ParseError::message("malformed timestamp"));
	};

	let Some(rest) = rest.strip_suffix(']') else {
		return Err(Lyrics3v2ParseError::message("malformed timestamp"));
	};

	let Some((mins, secs)) = rest.split_once(':') else {
		return Err(Lyrics3v2ParseError::message("malformed timestamp"));
	};

	let (mins_bytes, mins) = ascii_number(mins.as_bytes());
	if mins_bytes != 2 || mins > 59 {
		return Err(Lyrics3v2ParseError::message("malformed timestamp"));
	}

	let (secs_bytes, secs) = ascii_number(secs.as_bytes());
	if secs_bytes != 2 || secs > 59 {
		return Err(Lyrics3v2ParseError::message("malformed timestamp"));
	}

	Ok(Some(Timestamp {
		minute: Some(mins as u8),
		second: Some(secs as u8),
		..Timestamp::default()
	}))
}

impl Lyrics3v2Tag {
	pub(crate) fn parse(
		reader: &[u8],
		parse_options: ParseOptions,
	) -> Result<Self, Lyrics3v2ParseError> {
		log::debug!("Parsing Lyrics3v2 tag");

		let mut tag = Self::default();

		if reader.len() < (LYRICS3V2_HEADER.len() + LYRICS3V2_FOOTER.len())
			|| &reader[..LYRICS3V2_HEADER.len()] != LYRICS3V2_HEADER.as_bytes()
			|| !reader.ends_with(LYRICS3V2_FOOTER.as_bytes())
		{
			return Err(FakeTagError.into());
		}

		let lyrics_bytes = &mut &reader[..reader.len() - LYRICS3V2_FOOTER.len()];
		if lyrics_bytes.len() < 6 {
			return Err(FakeTagError.into());
		}

		let (tag_size_bytes, tag_size) = ascii_number(&lyrics_bytes[lyrics_bytes.len() - 6..]);
		if tag_size_bytes != 6 {
			return Err(Lyrics3v2ParseError::message("non-numeric tag size"));
		}

		let lyrics_bytes = &mut &lyrics_bytes[..lyrics_bytes.len() - 6];
		if tag_size as usize != lyrics_bytes.len() {
			if parse_options.parsing_mode == ParsingMode::Strict {
				return Err(SizeMismatchError.into());
			}

			log::warn!("Lyrics3v2 tag has the wrong size, attempting to parse anyway")
		}

		let lyrics_bytes = &mut &lyrics_bytes[LYRICS3V2_HEADER.len()..];
		while !lyrics_bytes.is_empty() {
			let header = field(lyrics_bytes)?;
			match header.id {
				FIELD_INDICATORS => {
					fn indication(
						c: u8,
						parse_mode: ParsingMode,
					) -> Result<bool, Lyrics3v2ParseError> {
						match c {
							b'1' => Ok(true),
							b'0' => Ok(false),
							_ => {
								if parse_mode == ParsingMode::Strict {
									return Err(Lyrics3v2ParseError::message(
										"invalid indication value",
									));
								}

								Ok(true)
							},
						}
					}

					let indications_str =
						parse_text(header, lyrics_bytes, 3, parse_options.parsing_mode)?;
					if indications_str.len() != 3 {
						if parse_options.parsing_mode == ParsingMode::Strict {
							return Err(Lyrics3v2ParseError::message("invalid indication value"));
						}

						log::warn!(
							"malformed Lyrics3v2 indications field, tag flags may be incorrect"
						);
						continue;
					}

					let indications = indications_str.as_bytes();
					tag.flags.synchronized_lyrics =
						indication(indications[1], parse_options.parsing_mode)?;
					tag.flags.random_selection =
						indication(indications[2], parse_options.parsing_mode)?;
				},
				FIELD_LYRICS => {
					tag.lyrics = Some(parse_text(
						header,
						lyrics_bytes,
						FIELD_MAX_SIZE,
						parse_options.parsing_mode,
					)?);
				},
				FIELD_INFO => {
					tag.info = Some(parse_text(
						header,
						lyrics_bytes,
						FIELD_MAX_SIZE,
						parse_options.parsing_mode,
					)?);
				},
				FIELD_AUTHOR => {
					tag.author = Some(parse_text(
						header,
						lyrics_bytes,
						BASIC_TEXT_FIELD_MAX_SIZE,
						parse_options.parsing_mode,
					)?);
				},
				FIELD_EXTENDED_ALBUM_NAME => {
					tag.extended_album_name = Some(parse_text(
						header,
						lyrics_bytes,
						BASIC_TEXT_FIELD_MAX_SIZE,
						parse_options.parsing_mode,
					)?);
				},
				FIELD_EXTENDED_ARTIST_NAME => {
					tag.extended_artist_name = Some(parse_text(
						header,
						lyrics_bytes,
						BASIC_TEXT_FIELD_MAX_SIZE,
						parse_options.parsing_mode,
					)?);
				},
				FIELD_EXTENDED_TRACK_TITLE => {
					tag.extended_track_title = Some(parse_text(
						header,
						lyrics_bytes,
						BASIC_TEXT_FIELD_MAX_SIZE,
						parse_options.parsing_mode,
					)?);
				},
				FIELD_IMAGE => {
					let images_str = parse_text(
						header,
						lyrics_bytes,
						FIELD_MAX_SIZE,
						parse_options.parsing_mode,
					)?;

					for image in images_str.lines() {
						let mut parts = image.split("||");
						let Some(filename) = parts.next() else {
							if parse_options.parsing_mode == ParsingMode::Strict {
								return Err(Lyrics3v2ParseError::message(
									"image field missing a filename",
								));
							}

							// Maybe just a single broken entry? keep trying...
							continue;
						};

						// The other fields should also be present, as the delimiters are required

						let description = match parts.next() {
							Some(description) if !description.is_empty() => Some(description),
							None if parse_options.parsing_mode == ParsingMode::Strict => {
								return Err(Lyrics3v2ParseError::message(
									"image field missing a description",
								));
							},
							_ => None,
						};

						let timestamp = match parts.next() {
							Some(timestamp) if !timestamp.is_empty() => {
								match parse_timestamp(timestamp) {
									Ok(Some(timestamp)) => Some(timestamp),
									Err(e) if parse_options.parsing_mode == ParsingMode::Strict => {
										return Err(e);
									},
									_ => None,
								}
							},
							None if parse_options.parsing_mode == ParsingMode::Strict => {
								return Err(Lyrics3v2ParseError::message(
									"image field missing a timestamp",
								));
							},
							_ => None,
						};

						tag.images.push(Image {
							filename: filename.to_string(),
							description: description.map(ToString::to_string),
							timestamp,
						})
					}
				},
				_ => {
					log::warn!(
						"unknown Lyrics3v2 field `{}`, skipping",
						header.id.escape_ascii()
					);

					if lyrics_bytes.len() < header.size as usize {
						return Err(SizeMismatchError.into());
					}
					*lyrics_bytes = &lyrics_bytes[header.size as usize..];
				},
			}
		}

		Ok(tag)
	}
}
