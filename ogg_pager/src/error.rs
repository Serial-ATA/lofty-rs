use std::error::Error;
use std::fmt;

/// Alias for `Result<T, PageError>`
pub type Result<T> = std::result::Result<T, PageError>;

/// Errors that can occur while performing `Page` operations
#[derive(Debug)]
pub enum PageError {
	/// The reader contains a page with a nonzero version
	InvalidVersion,
	/// The reader contains a page with a segment count < 1
	BadSegmentCount,
	/// The reader contains a page without a magic signature (OggS)
	MissingMagic,
	/// The reader contains too much data for a single page
	TooMuchData,
	/// The reader contains too little data to extract the expected information
	NotEnoughData,
	/// Any std::io::Error
	Io(std::io::Error),
}

impl fmt::Display for PageError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			PageError::InvalidVersion => {
				write!(f, "Invalid stream structure version (Should always be 0)")
			},
			PageError::BadSegmentCount => write!(f, "Page has a segment count < 1"),
			PageError::MissingMagic => write!(f, "Page is missing a magic signature"),
			PageError::TooMuchData => write!(f, "Too much data was provided"),
			PageError::NotEnoughData => {
				write!(f, "Too little data is available for the expected read")
			},
			PageError::Io(err) => write!(f, "{}", err),
		}
	}
}

impl Error for PageError {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		match *self {
			PageError::Io(ref e) => Some(e),
			_ => None,
		}
	}
}

impl From<std::io::Error> for PageError {
	fn from(err: std::io::Error) -> PageError {
		PageError::Io(err)
	}
}
