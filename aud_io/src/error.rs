use std::collections::TryReserveError;
use std::fmt::Display;

pub type Result<T> = std::result::Result<T, AudioError>;

#[derive(Debug)]
pub enum AudioError {
	// File data related errors
	/// Attempting to read/write an abnormally large amount of data
	TooMuchData,
	/// Expected the data to be a different size than provided
	///
	/// This occurs when the size of an item is written as one value, but that size is either too
	/// big or small to be valid within the bounds of that item.
	// TODO: Should probably have context
	SizeMismatch,

	/// Errors that arise while decoding text
	TextDecode(&'static str),
	/// Arises when an atom contains invalid data
	BadAtom(&'static str),

	// Conversions for external errors
	/// Unable to convert bytes to a String
	StringFromUtf8(std::string::FromUtf8Error),
	/// Unable to convert bytes to a str
	StrFromUtf8(std::str::Utf8Error),
	/// Represents all cases of [`std::io::Error`].
	Io(std::io::Error),
	/// Represents all cases of [`std::fmt::Error`].
	Fmt(std::fmt::Error),
	/// Failure to allocate enough memory
	Alloc(TryReserveError),
	/// This should **never** be encountered
	Infallible(std::convert::Infallible),
}

impl Display for AudioError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			// Conversions
			AudioError::StringFromUtf8(err) => write!(f, "{err}"),
			AudioError::StrFromUtf8(err) => write!(f, "{err}"),
			AudioError::Io(err) => write!(f, "{err}"),
			AudioError::Fmt(err) => write!(f, "{err}"),
			AudioError::Alloc(err) => write!(f, "{err}"),
			AudioError::Infallible(_) => write!(f, "A expected condition was not upheld"),

			AudioError::TextDecode(message) => write!(f, "Text decoding: {message}"),
			AudioError::BadAtom(message) => write!(f, "MP4 Atom: {message}"),

			// Files
			AudioError::TooMuchData => write!(
				f,
				"Attempted to read/write an abnormally large amount of data"
			),
			AudioError::SizeMismatch => write!(
				f,
				"Encountered an invalid item size, either too big or too small to be valid"
			),
		}
	}
}

impl core::error::Error for AudioError {}

impl From<std::io::Error> for AudioError {
	fn from(input: std::io::Error) -> Self {
		AudioError::Io(input)
	}
}

impl From<std::fmt::Error> for AudioError {
	fn from(input: std::fmt::Error) -> Self {
		AudioError::Fmt(input)
	}
}

impl From<std::string::FromUtf8Error> for AudioError {
	fn from(input: std::string::FromUtf8Error) -> Self {
		AudioError::StringFromUtf8(input)
	}
}

impl From<std::str::Utf8Error> for AudioError {
	fn from(input: std::str::Utf8Error) -> Self {
		AudioError::StrFromUtf8(input)
	}
}

impl From<TryReserveError> for AudioError {
	fn from(input: TryReserveError) -> Self {
		AudioError::Alloc(input)
	}
}

impl From<std::convert::Infallible> for AudioError {
	fn from(input: std::convert::Infallible) -> Self {
		AudioError::Infallible(input)
	}
}
