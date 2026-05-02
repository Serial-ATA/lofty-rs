//! [`VorbisComments`] error types
//!
//! [`VorbisComments`]: crate::ogg::tag::VorbisComments

use crate::error::{TagEncodingError, TagParseError};
use crate::tag::TagType;

use std::borrow::Cow;

use lofty_attr::LoftyError;

/// Internal concrete variant of [`TagParseError`] for conversions
#[derive(LoftyError)]
#[error(message = "failed to parse Vorbis Comments tag")]
pub(crate) struct VorbisCommentsParseError {
	#[error(from(
		std::io::Error,
		crate::error::SizeMismatchError,
		crate::error::AllocationError,
		crate::util::text::TextDecodingError,
		crate::picture::error::PictureParseError,
	))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl VorbisCommentsParseError {
	pub(super) fn message(message: impl Into<Cow<'static, str>>) -> Self {
		let message = message.into();
		Self {
			source: message.into(),
		}
	}
}

impl From<VorbisCommentsParseError> for TagParseError {
	fn from(input: VorbisCommentsParseError) -> Self {
		TagParseError::new(TagType::VorbisComments, input.source)
	}
}

/// Internal concrete variant of [`TagEncodingError`] for conversions
#[derive(LoftyError)]
#[error(message = "failed to parse Vorbis Comments tag")]
pub(crate) struct VorbisCommentsEncodingError {
	#[error(from(std::io::Error, crate::error::TooMuchDataError,))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl From<VorbisCommentsEncodingError> for TagEncodingError {
	fn from(input: VorbisCommentsEncodingError) -> Self {
		TagEncodingError::new(TagType::VorbisComments, input.source)
	}
}
