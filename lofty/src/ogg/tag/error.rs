//! [`VorbisComments`] error types
//!
//! [`VorbisComments`]: crate::ogg::tag::VorbisComments

use std::borrow::Cow;

use lofty_attr::LoftyError;

/// Failed to parse [`VorbisComments`]
///
/// [`VorbisComments`]: crate::ogg::tag::VorbisComments
#[derive(LoftyError)]
#[error(message = "failed to parse Vorbis Comments tag")]
pub struct VorbisCommentsParseError {
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
