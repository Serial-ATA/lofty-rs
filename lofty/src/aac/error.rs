//! AAC error types

use crate::error::FileParseError;
use crate::file::FileType;

use lofty_attr::LoftyError;

/// Failed to parse an [`AacFile`]
///
/// [`AacFile`]: crate::aac::AacFile
#[derive(LoftyError)]
#[error(message = "failed to parse AAC file")]
pub struct AacParseError {
	#[error(from(
		std::io::Error,
		crate::id3::v2::error::Id3v2ParseError,
		crate::id3::v1::error::Id3v1ParseError,
		crate::error::SizeMismatchError,
		crate::error::TooMuchDataError,
	))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl AacParseError {
	pub(super) fn message(message: &'static str) -> Self {
		Self {
			source: message.into(),
		}
	}
}

impl From<AacParseError> for FileParseError {
	fn from(input: AacParseError) -> FileParseError {
		Self::new(FileType::Aac, input.source)
	}
}
