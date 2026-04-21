//! WAV file/tag error types

use crate::error::FileParseError;
use crate::file::FileType;

use lofty_attr::LoftyError;

// Exports

pub use super::tag::error::RiffInfoListParseError;

/// Failed to parse a [`WavFile`]
///
/// [`WavFile`]: crate::iff::wav::WavFile
#[derive(LoftyError)]
#[error(message = "failed to parse WAV file")]
pub struct WavParseError {
	#[error(from(
		std::io::Error,
		crate::id3::v2::error::Id3v2ParseError,
		RiffInfoListParseError,
		crate::iff::error::ChunkParseError,
		crate::error::SizeMismatchError,
		crate::error::UnknownFormatError,
		crate::error::LoftyError, // TODO: Remove this
	))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl From<WavParseError> for FileParseError {
	fn from(input: WavParseError) -> FileParseError {
		Self::new(FileType::Wav, input.source)
	}
}
