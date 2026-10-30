use crate::error::{TagEncodingError, TagParseError};
use crate::tag::TagType;
use std::borrow::Cow;

use lofty_attr::LoftyError;

/// Internal concrete variant of [`TagParseError`] for conversions
#[derive(LoftyError)]
#[error(message = "failed to parse Lyrics3v2 tag")]
pub(crate) struct Lyrics3v2ParseError {
	#[error(from(
		std::io::Error,
		crate::error::TooMuchDataError,
		crate::error::SizeMismatchError,
		crate::error::FakeTagError,
		crate::util::alloc::AllocationError,
	))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl Lyrics3v2ParseError {
	pub(super) fn message(message: &'static str) -> Self {
		Self {
			source: message.into(),
		}
	}
}

impl From<Lyrics3v2ParseError> for TagParseError {
	fn from(input: Lyrics3v2ParseError) -> Self {
		TagParseError::new(TagType::Lyrics3v2, input.source)
	}
}

/// Internal concrete variant of [`TagEncodingError`] for conversions
#[derive(LoftyError)]
#[error(message = "failed to write Lyrics3v2 tag")]
pub(crate) struct Lyrics3v2EncodingError {
	#[error(from(
		std::io::Error,
		crate::error::TooMuchDataError,
		crate::error::FakeTagError,
		crate::util::text::TextEncodingError,
		crate::util::alloc::AllocationError,
	))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl Lyrics3v2EncodingError {
	pub(super) fn message(message: impl Into<Cow<'static, str>>) -> Self {
		let message = message.into();
		Self {
			source: message.into(),
		}
	}
}

impl From<Lyrics3v2EncodingError> for TagEncodingError {
	fn from(input: Lyrics3v2EncodingError) -> Self {
		TagEncodingError::new(TagType::Lyrics3v2, input.source)
	}
}
