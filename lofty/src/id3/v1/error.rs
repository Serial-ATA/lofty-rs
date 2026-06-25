//! ID3v1 error types

use crate::error::{TagEncodingError, TagParseError};
use crate::tag::TagType;

use lofty_attr::LoftyError;

/// Internal concrete variant of [`TagParseError`] for conversions
#[derive(LoftyError)]
#[error(message = "failed to parse ID3v1 tag")]
#[doc(hidden)] // Used in tests
pub struct Id3v1ParseError {
	#[error(from(std::io::Error, crate::error::FakeTagError,))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl Id3v1ParseError {
	pub(super) fn non_digit_year() -> Self {
		Self::message("expected 4 digit year field")
	}

	fn message(message: &'static str) -> Self {
		Self {
			source: message.into(),
		}
	}
}

impl From<Id3v1ParseError> for TagParseError {
	fn from(input: Id3v1ParseError) -> Self {
		TagParseError::new(TagType::Id3v1, input.source)
	}
}

/// Internal concrete variant of [`TagEncodingError`] for conversions
#[derive(LoftyError)]
#[error(message = "failed to write ID3v1 tag")]
pub(crate) struct Id3v1EncodingError {
	#[error(from(
		std::io::Error,
		crate::error::FakeTagError,
		crate::util::text::TextEncodingError,
	))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl From<Id3v1EncodingError> for TagEncodingError {
	fn from(input: Id3v1EncodingError) -> Self {
		TagEncodingError::new(TagType::Id3v1, input.source)
	}
}
