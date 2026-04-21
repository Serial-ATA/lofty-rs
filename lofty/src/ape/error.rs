//! APE file/tag error types

use crate::error::FileParseError;
use crate::file::FileType;

use lofty_attr::LoftyError;

// Exports

pub use super::tag::error::{ApeTagEncodingError, ApeTagItemValidationError, ApeTagParseError};

/// Failed to parse an [`ApeFile`]
///
/// [`ApeFile`]: crate::ape::ApeFile
#[derive(LoftyError)]
#[error(message = "failed to parse APE file")]
pub struct ApeParseError {
	#[error(from(
		std::io::Error,
		crate::id3::v2::error::Id3v2ParseError,
		crate::id3::v1::error::Id3v1ParseError,
		crate::id3::Lyrics3v2ParseError,
		ApeTagParseError,
		crate::error::SizeMismatchError,
		crate::error::FakeTagError,
		crate::error::LoftyError, // TODO: Remove this
	))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl ApeParseError {
	pub(super) fn message(message: &'static str) -> Self {
		Self {
			source: message.into(),
		}
	}
}

impl From<ApeParseError> for FileParseError {
	fn from(input: ApeParseError) -> FileParseError {
		Self::new(FileType::Ape, input.source)
	}
}
