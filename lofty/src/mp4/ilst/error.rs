use std::borrow::Cow;

use lofty_attr::LoftyError;

/// Failed to parse an [`Ilst`]
///
/// [`Ilst`]: crate::mp4::Ilst
#[derive(LoftyError)]
#[error(message = "failed to parse ilst tag")]
pub struct IlstParseError {
	#[error(from(
		std::io::Error,
		crate::mp4::error::AtomParseError,
		crate::error::AllocationError,
	))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

/// Failed to write an [`Ilst`]
///
/// [`Ilst`]: crate::mp4::Ilst
#[derive(LoftyError)]
#[error(message = "failed to write ilst tag")]
pub struct IlstEncodingError {
	#[error(from(
		std::io::Error,
		crate::error::LoftyError, // TODO: remove this
	))]
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
