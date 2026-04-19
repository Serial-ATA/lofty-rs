//! ID3v1 error types

use crate::error::{ErrorKind, LoftyError};

use std::fmt::Formatter;

/// Errors that can occur within ID3v1 tags
#[derive(Debug)]
pub enum Id3v1ParseError {
	/// **(STRICT MODE ONLY)** The `year` field isn't 4 digits.
	NonDigitYear,
}

impl core::fmt::Display for Id3v1ParseError {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Id3v1ParseError::NonDigitYear => write!(f, "expected 4 digit year field"),
		}
	}
}

impl core::error::Error for Id3v1ParseError {}

impl From<Id3v1ParseError> for LoftyError {
	fn from(input: Id3v1ParseError) -> Self {
		Self::new(ErrorKind::TagParse(input.into()))
	}
}
