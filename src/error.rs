//! Contains the errors that can arise within Lofty
//!
//! The primary error is [`LoftyError`]. The type of error is determined by [`ErrorKind`],
//! which can be extended at any time.

use crate::file::FileType;

use std::collections::TryReserveError;
use std::fmt::{Debug, Display, Formatter};

use ogg_pager::PageError;

/// Alias for `Result<T, LoftyError>`
pub type Result<T> = std::result::Result<T, LoftyError>;

#[derive(Debug)]
#[non_exhaustive]
/// The types of errors that can occur
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
	/// Errors that arise while reading/writing ID3v2 tags
	ID3v2(ID3v2Error),

	/// Arises when an atom contains invalid data
	BadAtom(&'static str),

	// Conversions for external errors
	/// Errors that arise while parsing OGG pages
	OggPage(ogg_pager::PageError),
	/// Unable to convert bytes to a String
	StringFromUtf8(std::string::FromUtf8Error),
	/// Unable to convert bytes to a str
	StrFromUtf8(std::str::Utf8Error),
	/// Represents all cases of [`std::io::Error`].
	Io(std::io::Error),
	/// Failure to allocate enough memory
	Alloc(TryReserveError),
}

#[derive(Debug, Clone)]
#[non_exhaustive]
/// The types of errors that can occur while interacting with ID3v2 tags
pub enum ID3v2ErrorKind {
	#[cfg(feature = "id3v2")]
	/// Arises when an invalid picture format is parsed. Only applicable to [`ID3v2Version::V2`](crate::id3::v2::ID3v2Version::V2)
	BadPictureFormat(String),
	/// Arises when an invalid ID3v2 version is found
	BadId3v2Version(u8, u8),
	#[cfg(feature = "id3v2")]
	/// Arises when a frame ID contains invalid characters (must be within `'A'..'Z'` or `'0'..'9'`)
	BadFrameID,
	#[cfg(feature = "id3v2")]
	/// Arises when a frame doesn't have enough data
	BadFrameLength,
	#[cfg(feature = "id3v2")]
	/// Arises when invalid data is encountered while reading an ID3v2 synchronized text frame
	BadSyncText,
	#[cfg(feature = "id3v2")]
	/// Arises when attempting to write an invalid Frame (Bad `FrameID`/`FrameValue` pairing)
	BadFrame(String, &'static str),
	/// A catch-all for all remaining errors
	///
	/// NOTE: This will likely be deprecated in the future
	Other(&'static str),
}

impl Display for ID3v2ErrorKind {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			ID3v2ErrorKind::BadId3v2Version(major, minor) => write!(
				f,
				"Found an invalid version (v{major}.{minor}), expected any major revision in: (2, \
				 3, 4)"
			),
			#[cfg(feature = "id3v2")]
			ID3v2ErrorKind::BadFrameID => write!(f, "Failed to parse a frame ID"),
			#[cfg(feature = "id3v2")]
			ID3v2ErrorKind::BadFrameLength => write!(
				f,
				"Frame isn't long enough to extract the necessary information"
			),
			#[cfg(feature = "id3v2")]
			ID3v2ErrorKind::BadSyncText => write!(f, "Encountered invalid data in SYLT frame"),
			#[cfg(feature = "id3v2")]
			ID3v2ErrorKind::BadFrame(ref frame_id, frame_value) => write!(
				f,
				"Attempted to write an invalid frame. ID: \"{}\", Value: \"{}\"",
				frame_id, frame_value
			),
			#[cfg(feature = "id3v2")]
			ID3v2ErrorKind::BadPictureFormat(format) => {
				write!(f, "Picture: Found unexpected format \"{format}\"")
			},
			ID3v2ErrorKind::Other(message) => write!(f, "{message}"),
		}
	}
}

/// An error that arises while interacting with an ID3v2 tag
pub struct ID3v2Error {
	kind: ID3v2ErrorKind,
}

impl ID3v2Error {
	/// Create a new `ID3v2Error` from an [`ID3v2ErrorKind`]
	pub fn new(kind: ID3v2ErrorKind) -> Self {
		Self { kind }
	}

	/// Returns the [`ID3v2ErrorKind`]
	pub fn kind(&self) -> ID3v2ErrorKind {
		self.kind.clone()
	}
}

impl Debug for ID3v2Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "ID3v2: {:?}", self.kind)
	}
}

impl Display for ID3v2Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "ID3v2: {}", self.kind)
	}
}

/// An error that arises while decoding a file
pub struct FileDecodingError {
	format: Option<FileType>,
	description: &'static str,
}

impl FileDecodingError {
	/// Create a `FileDecodingError` from a [`FileType`] and description
	pub fn new(format: FileType, description: &'static str) -> Self {
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
	pub fn new(format: FileType, description: &'static str) -> Self {
		Self {
			format: Some(format),
			description,
		}
	}

	/// Create a `FileEncodingError` without binding it to a [`FileType`]
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
	kind: ErrorKind,
}

impl LoftyError {
	/// Create a `LoftyError` from an [`ErrorKind`]
	pub fn new(kind: ErrorKind) -> Self {
		Self { kind }
	}

	/// Returns the [`ErrorKind`]
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

impl From<ID3v2Error> for LoftyError {
	fn from(input: ID3v2Error) -> Self {
		Self {
			kind: ErrorKind::ID3v2(input),
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

impl Display for LoftyError {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self.kind {
			// Conversions
			ErrorKind::OggPage(ref err) => write!(f, "{err}"),
			ErrorKind::StringFromUtf8(ref err) => write!(f, "{err}"),
			ErrorKind::StrFromUtf8(ref err) => write!(f, "{err}"),
			ErrorKind::Io(ref err) => write!(f, "{err}"),
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
			ErrorKind::ID3v2(ref id3v2_err) => write!(f, "{id3v2_err}"),
			ErrorKind::BadAtom(message) => write!(f, "MP4 Atom: {message}"),

			// Files
			ErrorKind::TooMuchData => write!(
				f,
				"An abnormally large amount of data was provided, and an overflow occurred"
			),
			ErrorKind::SizeMismatch => write!(
				f,
				"Encountered an invalid item size, either too big or too small to be valid"
			),
			ErrorKind::FileDecoding(ref file_decode_err) => write!(f, "{file_decode_err}"),
			ErrorKind::FileEncoding(ref file_encode_err) => write!(f, "{file_encode_err}"),
		}
	}
}
