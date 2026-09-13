//! DSF error types

use crate::error::FileParseError;
use crate::file::FileType;

use std::borrow::Cow;

use lofty_attr::LoftyError;

/// Failed to parse a [`DsfFile`]
///
/// [`DsfFile`]: crate::dsf::DsfFile
#[derive(LoftyError)]
#[error(message = "failed to parse DSF file")]
pub struct DsfParseError {
	#[error(from(
		std::io::Error,
		crate::error::TagParseError,
		crate::error::SizeMismatchError,
		crate::error::TooMuchDataError,
	))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl DsfParseError {
	pub(super) fn message(message: impl Into<Cow<'static, str>>) -> Self {
		let message = message.into();
		Self {
			source: message.into(),
		}
	}
}

impl From<DsfParseError> for FileParseError {
	fn from(input: DsfParseError) -> FileParseError {
		Self::new(FileType::Dsf, input.source)
	}
}
