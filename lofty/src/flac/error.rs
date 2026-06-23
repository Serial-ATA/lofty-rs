//! FLAC error types

use crate::error::{FileEncodingError, FileParseError};
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
		crate::error::TagParseError,
		crate::error::SizeMismatchError,
		crate::picture::error::PictureParseError,
		crate::error::AllocationError,
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

/// Internal concrete variant of [`FileEncodingError`] for conversions
#[derive(LoftyError)]
#[error(message = "failed to write to FLAC file")]
pub(super) struct FlacEncodingError {
	#[error(from(
		std::io::Error,
		crate::error::TooMuchDataError,
		crate::error::AllocationError,
		crate::error::TagEncodingError,
	))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl From<FlacEncodingError> for FileEncodingError {
	fn from(input: FlacEncodingError) -> FileEncodingError {
		Self::new(FileType::Flac, input.source)
	}
}
