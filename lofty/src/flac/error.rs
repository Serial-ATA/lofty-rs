//! FLAC error types

use crate::error::FileParseError;
use crate::file::FileType;

use lofty_attr::LoftyError;

/// Failed to parse a [`FlacFile`]
///
/// [`FlacFile`]: crate::flac::FlacFile
#[derive(LoftyError)]
#[error(message = "failed to parse FLAC file")]
pub struct FlacParseError {
	#[error(from(
		std::io::Error,
		crate::id3::v2::error::Id3v2ParseError,
		crate::error::SizeMismatchError,
		crate::error::LoftyError, // TODO: Remove this
	))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl FlacParseError {
	pub(super) fn message(message: &'static str) -> Self {
		Self {
			source: message.into(),
		}
	}
}

impl From<FlacParseError> for FileParseError {
	fn from(input: FlacParseError) -> FileParseError {
		Self::new(FileType::Flac, input.source)
	}
}
