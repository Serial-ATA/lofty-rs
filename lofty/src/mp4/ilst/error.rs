use crate::error::{TagEncodingError, TagParseError};
use crate::tag::TagType;

use std::borrow::Cow;

use lofty_attr::LoftyError;

/// Internal concrete variant of [`TagParseError`] for conversions
#[derive(LoftyError)]
#[error(message = "failed to parse ilst tag")]
pub(crate) struct IlstParseError {
	#[error(from(
		std::io::Error,
		crate::mp4::error::AtomParseError,
		crate::error::AllocationError,
	))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl From<IlstParseError> for TagParseError {
	fn from(input: IlstParseError) -> Self {
		TagParseError::new(TagType::Mp4Ilst, input.source)
	}
}

/// Internal concrete variant of [`TagEncodingError`] for conversions
#[derive(LoftyError)]
#[error(message = "failed to write ilst tag")]
pub(crate) struct IlstEncodingError {
	#[error(from(std::io::Error,))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl IlstEncodingError {
	pub(super) fn message(message: impl Into<Cow<'static, str>>) -> Self {
		let message = message.into();
		Self {
			source: message.into(),
		}
	}
}

impl From<IlstEncodingError> for TagEncodingError {
	fn from(input: IlstEncodingError) -> Self {
		TagEncodingError::new(TagType::Mp4Ilst, input.source)
	}
}
