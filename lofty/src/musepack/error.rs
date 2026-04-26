//! MusePack error types

use crate::error::FileParseError;
use crate::file::FileType;

use lofty_attr::LoftyError;

/// Failed to parse an [`MpcFile`]
///
/// [`MpcFile`]: crate::musepack::MpcFile
#[derive(LoftyError)]
#[error(message = "failed to parse MusePack file")]
pub struct MpcParseError {
	#[error(from(
		std::io::Error,
		crate::ape::error::ApeTagParseError,
		crate::id3::v2::error::Id3v2ParseError,
		crate::id3::v1::error::Id3v1ParseError,
		crate::id3::Lyrics3v2ParseError,
		crate::error::FakeTagError,
		crate::error::SizeMismatchError,
		crate::error::TooMuchDataError,
	))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl MpcParseError {
	pub(super) fn message(message: &'static str) -> Self {
		Self {
			source: message.into(),
		}
	}
}

impl From<MpcParseError> for FileParseError {
	fn from(input: MpcParseError) -> FileParseError {
		Self::new(FileType::Mpc, input.source)
	}
}
