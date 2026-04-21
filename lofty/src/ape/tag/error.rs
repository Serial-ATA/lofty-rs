use crate::error::{AllocationError, TextDecodingError};

use std::borrow::Cow;

use lofty_attr::LoftyError;

/// Errors that can occur while validating an APE tag item
#[derive(Debug)]
pub struct ApeTagItemValidationError {
	message: Cow<'static, str>,
}

impl ApeTagItemValidationError {
	pub(super) fn illegal_key(key: &str) -> Self {
		Self::new(format!("key '{key}' is illegal"))
	}

	pub(super) fn invalid_length() -> Self {
		Self::new("item key has an invalid length (< 2 || > 255)")
	}

	pub(super) fn invalid_characters() -> Self {
		Self::new("item key contains invalid characters")
	}

	fn new(message: impl Into<Cow<'static, str>>) -> Self {
		let message = message.into();
		Self { message }
	}
}

impl core::fmt::Display for ApeTagItemValidationError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "{}", self.message)
	}
}

impl core::error::Error for ApeTagItemValidationError {}

/// Errors that can occur while parsing an APE tag item
pub struct ApeTagItemParseError {
	key: Option<String>,
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl ApeTagItemParseError {
	/// Get the name of the failed item, if it could be determined
	pub fn key(&self) -> Option<&str> {
		self.key.as_deref()
	}

	pub(super) fn illegal_item_type(key: String) -> Self {
		Self::message(key, "item contains an invalid item type")
	}

	pub(super) fn message(key: String, message: impl Into<Cow<'static, str>>) -> Self {
		let message = message.into();
		Self {
			key: Some(key),
			source: message.into(),
		}
	}
}

impl core::fmt::Debug for ApeTagItemParseError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("ApeTagItemParseError")
			.field("key", &self.key)
			.finish_non_exhaustive()
	}
}

impl core::fmt::Display for ApeTagItemParseError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self.key() {
			Some(key) => write!(f, "failed to parse APE tag item '{key}'"),
			None => write!(f, "failed to parse APE tag item"),
		}
	}
}

impl core::error::Error for ApeTagItemParseError {
	fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
		Some(&*self.source)
	}
}

impl From<(String, AllocationError)> for ApeTagItemParseError {
	fn from((key, error): (String, AllocationError)) -> Self {
		Self {
			key: Some(key),
			source: Box::new(error),
		}
	}
}

impl From<(String, std::io::Error)> for ApeTagItemParseError {
	fn from((key, error): (String, std::io::Error)) -> Self {
		Self {
			key: Some(key),
			source: Box::new(error),
		}
	}
}

impl From<(String, TextDecodingError)> for ApeTagItemParseError {
	fn from((key, error): (String, TextDecodingError)) -> Self {
		Self {
			key: Some(key),
			source: Box::new(error),
		}
	}
}

impl From<TextDecodingError> for ApeTagItemParseError {
	fn from(input: TextDecodingError) -> Self {
		Self {
			key: None,
			source: Box::new(input),
		}
	}
}

impl From<ApeTagItemValidationError> for ApeTagItemParseError {
	fn from(value: ApeTagItemValidationError) -> Self {
		Self {
			key: None,
			source: Box::new(value),
		}
	}
}

/// Errors that can occur while parsing an APE tag
#[derive(LoftyError)]
#[error(message = "failed to parse APE tag")]
pub struct ApeTagParseError {
	#[error(from(
		std::io::Error,
		crate::error::FakeTagError,
		crate::error::SizeMismatchError,
		crate::util::alloc::AllocationError,
		ApeTagItemParseError,
	))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

/// Errors that can occur while encoding an APE tag
#[derive(LoftyError)]
#[error(message = "failed to write APE tag")]
pub struct ApeTagEncodingError {
	#[error(from(std::io::Error, crate::util::alloc::AllocationError,))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}
