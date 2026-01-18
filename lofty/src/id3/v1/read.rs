use super::constants::{GENRES, ID3V1_TAG_MARKER};
use super::tag::Id3v1Tag;
use crate::config::ParsingMode;
use crate::error::LoftyError;
use crate::macros::err;
use crate::util::text::latin1_decode;

impl Id3v1Tag {
	/// This is **NOT** a public API
	#[doc(hidden)]
	pub fn parse(reader: [u8; 128], parse_mode: ParsingMode) -> Result<Self, LoftyError> {
		let mut tag = Self {
			title: None,
			artist: None,
			album: None,
			year: None,
			comment: None,
			track_number: None,
			genre: None,
		};

		if reader[..3] != ID3V1_TAG_MARKER {
			err!(FakeTag);
		}

		let reader = &reader[3..];

		tag.title = decode_text(&reader[..30]);
		tag.artist = decode_text(&reader[30..60]);
		tag.album = decode_text(&reader[60..90]);

		tag.year = try_parse_year(&reader[90..94], parse_mode)?;

		// Determine the range of the comment (30 bytes for ID3v1 and 28 for ID3v1.1)
		// We check for the null terminator 28 bytes in, and for a non-zero track number after it.
		// A track number of 0 is invalid.
		let range = if reader[122] == 0 && reader[123] != 0 {
			tag.track_number = Some(reader[123]);

			94_usize..123
		} else {
			94..124
		};

		tag.comment = decode_text(&reader[range]);

		if reader[124] < GENRES.len() as u8 {
			tag.genre = Some(reader[124]);
		}

		Ok(tag)
	}
}

fn decode_text(data: &[u8]) -> Option<String> {
	let mut first_null_pos = data.len();
	if let Some(null_pos) = data.iter().position(|&b| b == 0) {
		if null_pos == 0 {
			return None;
		}

		if data[null_pos..].iter().any(|b| *b != b'\0') {
			log::warn!("ID3v1 text field contains trailing junk, skipping");
		}

		first_null_pos = null_pos;
	}

	Some(latin1_decode(&data[..first_null_pos]))
}

fn try_parse_year(input: &[u8], parse_mode: ParsingMode) -> Result<Option<u16>, LoftyError> {
	let (num_digits, year) = input
		.iter()
		.take_while(|c| (**c).is_ascii_digit())
		.fold((0usize, 0u16), |(num_digits, year), c| {
			(num_digits + 1, year * 10 + u16::from(*c - b'0'))
		});
	if num_digits != 4 {
		// The official test suite says that any year that isn't 4 characters should be a decoding failure.
		// However, it seems most popular libraries (including us) will write "\0\0\0\0" for empty
		// years, rather than "0000" as the "spec" would suggest.
		if parse_mode == ParsingMode::Strict {
			err!(TextDecode(
				"ID3v1 year field contains non-ASCII digit characters"
			));
		}

		return Ok(None);
	}

	Ok(Some(year))
}
