use crate::error::{TagEncodingError, TagParseError};
use crate::tag::TagType;

use lofty_attr::LoftyError;

/// Internal concrete variant of [`TagParseError`] for conversions
#[derive(LoftyError)]
#[error(message = "failed to parse Lyrics3 tag")]
pub(crate) struct Lyrics3ParseError {
	#[error(from(
		std::io::Error,
		crate::error::FakeTagError,
		crate::util::alloc::AllocationError,
	))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl From<Lyrics3ParseError> for TagParseError {
	fn from(input: Lyrics3ParseError) -> Self {
		TagParseError::new(TagType::Lyrics3, input.source)
	}
}

/// Internal concrete variant of [`TagEncodingError`] for conversions
#[derive(LoftyError)]
#[error(message = "failed to write Lyrics3 tag")]
pub(crate) struct Lyrics3EncodingError {
	#[error(from(
		std::io::Error,
		crate::error::FakeTagError,
		crate::error::TooMuchDataError,
		crate::util::text::TextEncodingError,
		crate::util::alloc::AllocationError,
	))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl Lyrics3EncodingError {
	pub(super) fn message(message: &'static str) -> Self {
		Self {
			source: message.into(),
		}
	}
}

impl From<Lyrics3EncodingError> for TagEncodingError {
	fn from(input: Lyrics3EncodingError) -> Self {
		TagEncodingError::new(TagType::Lyrics3, input.source)
	}
}
