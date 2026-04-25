//! [`Picture`] error types
//!
//! [`Picture`]: crate::picture::Picture

use std::borrow::Cow;

use lofty_attr::LoftyError;

/// Unable to parse a [`Picture`]
///
/// [`Picture`]: crate::picture::Picture
#[derive(LoftyError)]
#[error(message = "failed to parse picture")]
pub struct PictureParseError {
	#[error(from(
		std::io::Error,
		crate::error::NotEnoughDataError,
		crate::error::SizeMismatchError,
		crate::error::AllocationError,
		crate::util::text::TextDecodingError,
		UnknownImageFormatError,
		data_encoding::DecodeError,
		crate::error::LoftyError, // TODO: Remove this
	))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl PictureParseError {
	pub(super) fn message(message: impl Into<Cow<'static, str>>) -> Self {
		let message = message.into();
		Self {
			source: message.into(),
		}
	}
}

/// Unable to determine the image format
#[derive(LoftyError)]
#[error(message = "unable to determine image format")]
pub struct UnknownImageFormatError;
