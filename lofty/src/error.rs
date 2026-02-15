//! Contains the errors that can arise within Lofty
//!
//! The primary error is [`LoftyError`]. The type of error is determined by [`ErrorKind`],
//! which can be extended at any time.

use crate::file::FileType;
use crate::id3::v2::error::Id3v2Error;
pub use crate::util::text::TextEncodingError;

use std::collections::TryReserveError;
use std::fmt::{Debug, Display, Formatter};

use ogg_pager::PageError;

/// Alias for `Result<T, LoftyError>`
pub type Result<T> = std::result::Result<T, LoftyError>;

/// The types of errors that can occur
#[derive(Debug)]
#[non_exhaustive]
pub enum ErrorKind {
	// File format related errors
	/// Unable to guess the format
	UnknownFormat,

	// File data related errors
	/// Attempting to read/write an abnormally large amount of data
	TooMuchData,
	/// Expected the data to be a different size than provided
	///
	/// This occurs when the size of an item is written as one value, but that size is either too
	/// big or small to be valid within the bounds of that item.
	// TODO: Should probably have context
	SizeMismatch,
	/// Errors that occur while decoding a file
	FileDecoding(FileDecodingError),
	/// Errors that occur while encoding a file
	FileEncoding(FileEncodingError),

	// Picture related errors
	/// Provided an invalid picture
	NotAPicture,
	/// Attempted to write a picture that the format does not support
	UnsupportedPicture,

	// Tag related errors
	/// Arises when writing a tag to a file type that doesn't support it
	UnsupportedTag,
	/// Arises when a tag is expected (Ex. found an "ID3 " chunk in a WAV file), but isn't found
	FakeTag,
	/// Errors that arise while decoding text
	TextDecode(&'static str),
	/// Errors that arise while encoding text
	TextEncode(TextEncodingError),
	/// Arises when decoding OR encoding a problematic [`Timestamp`](crate::tag::items::Timestamp)
	BadTimestamp(&'static str),
	/// Errors that arise while reading/writing ID3v2 tags
	Id3v2(Id3v2Error),

	/// Arises when an atom contains invalid data
	BadAtom(&'static str),
	/// Arises when attempting to use [`Atom::merge`](crate::mp4::Atom::merge) with mismatching identifiers
	AtomMismatch,

	// Conversions for external errors
	/// Errors that arise while parsing OGG pages
	OggPage(ogg_pager::PageError),
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

/// An error that arises while decoding a file
pub struct FileDecodingError {
	format: Option<FileType>,
	description: &'static str,
}

impl FileDecodingError {
	/// Create a `FileDecodingError` from a [`FileType`] and description
	#[must_use]
	pub const fn new(format: FileType, description: &'static str) -> Self {
		Self {
			format: Some(format),
			description,
		}
	}

	/// Create a `FileDecodingError` without binding it to a [`FileType`]
	pub fn from_description(description: &'static str) -> Self {
		Self {
			format: None,
			description,
		}
	}

	/// Returns the associated [`FileType`], if one exists
	pub fn format(&self) -> Option<FileType> {
		self.format
	}

	/// Returns the error description
	pub fn description(&self) -> &str {
		self.description
	}
}

impl Debug for FileDecodingError {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		if let Some(format) = self.format {
			write!(f, "{:?}: {:?}", format, self.description)
		} else {
			write!(f, "{:?}", self.description)
		}
	}
}

impl Display for FileDecodingError {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		if let Some(format) = self.format {
			write!(f, "{:?}: {}", format, self.description)
		} else {
			write!(f, "{}", self.description)
		}
	}
}

/// An error that arises while encoding a file
pub struct FileEncodingError {
	format: Option<FileType>,
	description: &'static str,
}

impl FileEncodingError {
	/// Create a `FileEncodingError` from a [`FileType`] and description
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::error::FileEncodingError;
	/// use lofty::file::FileType;
	///
	/// // This error is bounded to `FileType::Mpeg`, which will be displayed when the error is formatted
	/// let mpeg_error =
	/// 	FileEncodingError::new(FileType::Mpeg, "Something went wrong in the MPEG file!");
	/// ```
	#[must_use]
	pub const fn new(format: FileType, description: &'static str) -> Self {
		Self {
			format: Some(format),
			description,
		}
	}

	/// Create a `FileEncodingError` without binding it to a [`FileType`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::error::FileEncodingError;
	/// use lofty::file::FileType;
	///
	/// // The error isn't bounded to FileType::Mpeg, only the message will be displayed when the
	/// // error is formatted
	/// let mpeg_error = FileEncodingError::from_description("Something went wrong in the MPEG file!");
	/// ```
	pub fn from_description(description: &'static str) -> Self {
		Self {
			format: None,
			description,
		}
	}

	/// Returns the associated [`FileType`], if one exists
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::error::FileEncodingError;
	/// use lofty::file::FileType;
	///
	/// let mpeg_error =
	/// 	FileEncodingError::new(FileType::Mpeg, "Something went wrong in the MPEG file!");
	///
	/// assert_eq!(mpeg_error.format(), Some(FileType::Mpeg));
	/// ```
	pub fn format(&self) -> Option<FileType> {
		self.format
	}

	/// Returns the error description
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::error::FileEncodingError;
	/// use lofty::file::FileType;
	///
	/// let mpeg_error =
	/// 	FileEncodingError::new(FileType::Mpeg, "Something went wrong in the MPEG file!");
	///
	/// assert_eq!(
	/// 	mpeg_error.description(),
	/// 	"Something went wrong in the MPEG file!"
	/// );
	/// ```
	pub fn description(&self) -> &str {
		self.description
	}
}

impl Debug for FileEncodingError {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		if let Some(format) = self.format {
			write!(f, "{:?}: {:?}", format, self.description)
		} else {
			write!(f, "{:?}", self.description)
		}
	}
}

impl Display for FileEncodingError {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		if let Some(format) = self.format {
			write!(f, "{:?}: {:?}", format, self.description)
		} else {
			write!(f, "{}", self.description)
		}
	}
}

/// Errors that could occur within Lofty
pub struct LoftyError {
	pub(crate) kind: ErrorKind,
}

impl LoftyError {
	/// Create a `LoftyError` from an [`ErrorKind`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::error::{ErrorKind, LoftyError};
	///
	/// let unknown_format = LoftyError::new(ErrorKind::UnknownFormat);
	/// ```
	#[must_use]
	pub const fn new(kind: ErrorKind) -> Self {
		Self { kind }
	}

	/// Returns the [`ErrorKind`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::error::{ErrorKind, LoftyError};
	///
	/// let unknown_format = LoftyError::new(ErrorKind::UnknownFormat);
	/// if let ErrorKind::UnknownFormat = unknown_format.kind() {
	/// 	println!("What's the format?");
	/// }
	/// ```
	pub fn kind(&self) -> &ErrorKind {
		&self.kind
	}
}

impl std::error::Error for LoftyError {}

impl Debug for LoftyError {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:?}", self.kind)
	}
}

impl From<Id3v2Error> for LoftyError {
	fn from(input: Id3v2Error) -> Self {
		Self {
			kind: ErrorKind::Id3v2(input),
		}
	}
}

impl From<FileDecodingError> for LoftyError {
	fn from(input: FileDecodingError) -> Self {
		Self {
			kind: ErrorKind::FileDecoding(input),
		}
	}
}

impl From<FileEncodingError> for LoftyError {
	fn from(input: FileEncodingError) -> Self {
		Self {
			kind: ErrorKind::FileEncoding(input),
		}
	}
}

impl From<TextEncodingError> for LoftyError {
	fn from(input: TextEncodingError) -> Self {
		Self {
			kind: ErrorKind::TextEncode(input),
		}
	}
}

impl From<ogg_pager::PageError> for LoftyError {
	fn from(input: PageError) -> Self {
		Self {
			kind: ErrorKind::OggPage(input),
		}
	}
}

impl From<std::io::Error> for LoftyError {
	fn from(input: std::io::Error) -> Self {
		Self {
			kind: ErrorKind::Io(input),
		}
	}
}

impl From<std::fmt::Error> for LoftyError {
	fn from(input: std::fmt::Error) -> Self {
		Self {
			kind: ErrorKind::Fmt(input),
		}
	}
}

impl From<std::string::FromUtf8Error> for LoftyError {
	fn from(input: std::string::FromUtf8Error) -> Self {
		Self {
			kind: ErrorKind::StringFromUtf8(input),
		}
	}
}

impl From<std::str::Utf8Error> for LoftyError {
	fn from(input: std::str::Utf8Error) -> Self {
		Self {
			kind: ErrorKind::StrFromUtf8(input),
		}
	}
}

impl From<std::collections::TryReserveError> for LoftyError {
	fn from(input: TryReserveError) -> Self {
		Self {
			kind: ErrorKind::Alloc(input),
		}
	}
}

impl From<std::convert::Infallible> for LoftyError {
	fn from(input: std::convert::Infallible) -> Self {
		Self {
			kind: ErrorKind::Infallible(input),
		}
	}
}

impl Display for LoftyError {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self.kind {
			// Conversions
			ErrorKind::OggPage(ref err) => write!(f, "{err}"),
			ErrorKind::StringFromUtf8(ref err) => write!(f, "{err}"),
			ErrorKind::StrFromUtf8(ref err) => write!(f, "{err}"),
			ErrorKind::Io(ref err) => write!(f, "{err}"),
			ErrorKind::Fmt(ref err) => write!(f, "{err}"),
			ErrorKind::Alloc(ref err) => write!(f, "{err}"),

			ErrorKind::UnknownFormat => {
				write!(f, "No format could be determined from the provided file")
			},
			ErrorKind::NotAPicture => write!(f, "Picture: Encountered invalid data"),
			ErrorKind::UnsupportedPicture => {
				write!(f, "Picture: attempted to write an unsupported picture")
			},
			ErrorKind::UnsupportedTag => write!(
				f,
				"Attempted to write a tag to a format that does not support it"
			),
			ErrorKind::FakeTag => write!(f, "Reading: Expected a tag, found invalid data"),
			ErrorKind::TextDecode(message) => write!(f, "Text decoding: {message}"),
			ErrorKind::TextEncode(message) => write!(f, "Text encoding: {message}"),
			ErrorKind::BadTimestamp(message) => {
				write!(f, "Encountered an invalid timestamp: {message}")
			},
			ErrorKind::Id3v2(ref id3v2_err) => write!(f, "{id3v2_err}"),
			ErrorKind::BadAtom(message) => write!(f, "MP4 Atom: {message}"),
			ErrorKind::AtomMismatch => write!(
				f,
				"MP4 Atom: Attempted to use `Atom::merge()` with mismatching identifiers"
			),

			// Files
			ErrorKind::TooMuchData => write!(
				f,
				"Attempted to read/write an abnormally large amount of data"
			),
			ErrorKind::SizeMismatch => write!(
				f,
				"Encountered an invalid item size, either too big or too small to be valid"
			),
			ErrorKind::FileDecoding(ref file_decode_err) => write!(f, "{file_decode_err}"),
			ErrorKind::FileEncoding(ref file_encode_err) => write!(f, "{file_encode_err}"),

			ErrorKind::Infallible(_) => write!(f, "A expected condition was not upheld"),
		}
	}
}
