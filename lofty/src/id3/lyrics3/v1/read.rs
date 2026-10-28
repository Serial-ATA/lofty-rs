use super::Lyrics3Tag;
use super::constants::{LYRICS3_FOOTER, LYRICS3_HEADER};
use super::errors::Lyrics3ParseError;
use crate::error::FakeTagError;
use crate::util::text::latin1_decode;

impl Lyrics3Tag {
	pub(crate) fn parse(reader: &[u8]) -> Result<Self, Lyrics3ParseError> {
		let mut tag = Self {
			lyrics: String::new(),
		};

		if reader.len() < (LYRICS3_HEADER.len() + LYRICS3_FOOTER.len())
			|| &reader[..LYRICS3_HEADER.len()] != LYRICS3_HEADER.as_bytes()
			|| !reader.ends_with(LYRICS3_FOOTER.as_bytes())
		{
			return Err(FakeTagError.into());
		}

		let lyrics_bytes = &reader[LYRICS3_HEADER.len()..reader.len() - LYRICS3_FOOTER.len()];
		tag.lyrics = latin1_decode(lyrics_bytes);

		Ok(tag)
	}
}
