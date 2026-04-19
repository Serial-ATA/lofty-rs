use crate::error::FileParseError;
use crate::file::FileType;

use lofty_attr::LoftyError;

/// Failed to parse an [`MpegFile`]
///
/// [`MpegFile`]: crate::mpeg::MpegFile
#[derive(LoftyError)]
#[error(message = "failed to parse MPEG file")]
pub struct MpegParseError {
	#[error(from(
		std::io::Error,
		crate::id3::v2::error::Id3v2ParseError,
		crate::id3::v1::error::Id3v1ParseError,
		crate::id3::Lyrics3v2ParseError,
		crate::error::FakeTagError,
		crate::error::SizeMismatchError,
		crate::error::LoftyError, // TODO: Remove this
	))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl MpegParseError {
	pub(super) fn message(message: &'static str) -> Self {
		Self {
			source: message.into(),
		}
	}
}

impl From<MpegParseError> for FileParseError {
	fn from(input: MpegParseError) -> FileParseError {
		Self::new(FileType::Mpeg, input.source)
	}
}
