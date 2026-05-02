use crate::error::{TagEncodingError, TagParseError};
use crate::tag::TagType;

use lofty_attr::LoftyError;

/// Internal concrete variant of [`TagParseError`] for conversions
#[derive(LoftyError)]
#[error(message = "failed to parse RIFF INFO tag")]
pub struct RiffInfoListParseError {
	#[error(from(
		crate::iff::error::ChunkParseError,
		crate::util::text::TextDecodingError,
	))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl RiffInfoListParseError {
	pub(super) fn invalid_fourcc(fourcc: [u8; 4]) -> Self {
		Self {
			source: format!("item key is not a valid FourCC: {}", fourcc.escape_ascii()).into(),
		}
	}
}

impl From<RiffInfoListParseError> for TagParseError {
	fn from(input: RiffInfoListParseError) -> Self {
		TagParseError::new(TagType::RiffInfo, input.source)
	}
}

/// Internal concrete variant of [`TagEncodingError`] for conversions
#[derive(LoftyError)]
#[error(message = "failed to write RIFF INFO tag")]
pub(crate) struct RiffInfoListEncodingError {
	#[error(from(
		std::io::Error,
		crate::error::FakeTagError,
		crate::error::TooMuchDataError,
	))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl From<RiffInfoListEncodingError> for TagEncodingError {
	fn from(input: RiffInfoListEncodingError) -> Self {
		TagEncodingError::new(TagType::RiffInfo, input.source)
	}
}
