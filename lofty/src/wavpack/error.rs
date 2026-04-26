//! WavPack error types

use crate::error::FileParseError;
use crate::file::FileType;

use lofty_attr::LoftyError;

/// Failed to parse a [`WavPackFile`]
///
/// [`WavPackFile`]: crate::wavpack::WavPackFile
#[derive(LoftyError)]
#[error(message = "failed to parse WavPack file")]
pub struct WavPackParseError {
	#[error(from(
		std::io::Error,
		crate::ape::error::ApeTagParseError,
		crate::id3::v1::error::Id3v1ParseError,
		crate::id3::Lyrics3v2ParseError,
		crate::error::SizeMismatchError,
		crate::error::AllocationError,
		crate::error::TooMuchDataError,
		crate::error::NotEnoughDataError,
		crate::error::UnknownFormatError,
	))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl WavPackParseError {
	pub(super) fn message(message: &'static str) -> Self {
		Self {
			source: message.into(),
		}
	}
}

impl From<WavPackParseError> for FileParseError {
	fn from(input: WavPackParseError) -> FileParseError {
		Self::new(FileType::WavPack, input.source)
	}
}
