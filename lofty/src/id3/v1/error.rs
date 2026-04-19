//! ID3v1 error types

use lofty_attr::LoftyError;

/// Errors that can occur within ID3v1 tags
#[derive(LoftyError)]
#[error(message = "failed to parse ID3v1 tag")]
pub struct Id3v1ParseError {
	#[error(from(std::io::Error, crate::error::FakeTagError,))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl Id3v1ParseError {
	pub(super) fn non_digit_year() -> Self {
		Self::message("expected 4 digit year field")
	}

	fn message(message: &'static str) -> Self {
		Self {
			source: message.into(),
		}
	}
}
