//! AIFF file/tag error types

use crate::error::FileParseError;
use crate::file::FileType;

use lofty_attr::LoftyError;

// Exports

pub use super::tag::error::AiffTextChunksParseError;

/// Failed to parse an [`AiffFile`]
///
/// [`AiffFile`]: crate::iff::aiff::AiffFile
#[derive(LoftyError)]
#[error(message = "failed to parse AIFF file")]
pub struct AiffParseError {
	#[error(from(
		std::io::Error,
		crate::id3::v2::error::Id3v2ParseError,
		AiffTextChunksParseError,
		crate::iff::error::ChunkParseError,
		crate::error::SizeMismatchError,
		crate::error::UnknownFormatError,
		crate::error::NotEnoughDataError,
		crate::error::AllocationError,
		crate::util::text::TextDecodingError,
	))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl AiffParseError {
	pub(super) fn missing_comm() -> Self {
		Self::message("file does not contain an \"COMM\" chunk")
	}

	pub(super) fn message(message: &'static str) -> Self {
		Self {
			source: message.into(),
		}
	}
}

impl From<AiffParseError> for FileParseError {
	fn from(input: AiffParseError) -> FileParseError {
		Self::new(FileType::Aiff, input.source)
	}
}
