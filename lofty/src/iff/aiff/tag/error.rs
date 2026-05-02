use crate::error::{TagEncodingError, TagParseError};
use crate::tag::TagType;

use lofty_attr::LoftyError;

/// Internal concrete variant of [`TagParseError`] for conversions
#[derive(LoftyError)]
#[error(message = "failed to parse AIFF text chunks tag")]
pub struct AiffTextChunksParseError {
	#[error(from(std::io::Error, crate::iff::error::ChunkParseError,))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl From<AiffTextChunksParseError> for TagParseError {
	fn from(input: AiffTextChunksParseError) -> Self {
		TagParseError::new(TagType::AiffText, input.source)
	}
}

/// Internal concrete variant of [`TagEncodingError`] for conversions
#[derive(LoftyError)]
#[error(message = "failed to write AIFF text chunks tag")]
pub(crate) struct AiffTextChunksEncodingError {
	#[error(from(
		std::io::Error,
		crate::iff::error::ChunkParseError,
		crate::error::TooMuchDataError
	))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl From<AiffTextChunksEncodingError> for TagEncodingError {
	fn from(input: AiffTextChunksEncodingError) -> Self {
		TagEncodingError::new(TagType::AiffText, input.source)
	}
}
