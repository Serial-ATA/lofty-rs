//! Contains the errors that can arise within Lofty
//!
//! The primary error is [`LoftyError`]. The type of error is determined by [`ErrorKind`],
//! which can be extended at any time.

use crate::file::FileType;
use crate::id3::v2::FrameId;
use crate::tag::ItemKey;
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

/// The types of errors that can occur while interacting with ID3v2 tags
#[derive(Debug)]
#[non_exhaustive]
pub enum Id3v2ErrorKind {
	// Header
	/// Arises when an invalid ID3v2 version is found
	BadId3v2Version(u8, u8),
	/// Arises when a compressed ID3v2.2 tag is encountered
	///
	/// At the time the ID3v2.2 specification was written, a compression scheme wasn't decided.
	/// As such, it is recommended to ignore the tag entirely.
	V2Compression,
	/// Arises when an extended header has an invalid size (must be >= 6 bytes and less than the total tag size)
	BadExtendedHeaderSize,

	// Frame
	/// Arises when a frame ID contains invalid characters (must be within `'A'..'Z'` or `'0'..'9'`)
	/// or if the ID is too short/long.
	BadFrameId(Vec<u8>),
	/// Arises when no frame ID is available in the ID3v2 specification for an item key
	/// and the associated value type.
	UnsupportedFrameId(ItemKey),
	/// Arises when a frame doesn't have enough data
	BadFrameLength,
	/// Arises when a frame with no content is parsed with [ParsingMode::Strict](crate::config::ParsingMode::Strict)
	EmptyFrame(FrameId<'static>),
	/// Arises when reading/writing a compressed or encrypted frame with no data length indicator
	MissingDataLengthIndicator,
	/// Arises when a frame or tag has its unsynchronisation flag set, but the content is not actually synchsafe
	///
	/// See [`FrameFlags::unsynchronisation`](crate::id3::v2::FrameFlags::unsynchronisation) for an explanation.
	InvalidUnsynchronisation,
	/// Arises when a text encoding other than Latin-1 or UTF-16 appear in an ID3v2.2 tag
	V2InvalidTextEncoding,
	/// Arises when an invalid picture format is parsed. Only applicable to [`ID3v2Version::V2`](crate::id3::v2::Id3v2Version::V2)
	BadPictureFormat(String),
	/// Arises when invalid data is encountered while reading an ID3v2 synchronized text frame
	BadSyncText,
	/// Arises when decoding a [`UniqueFileIdentifierFrame`](crate::id3::v2::UniqueFileIdentifierFrame) with no owner
	MissingUfidOwner,
	/// Arises when decoding a [`RelativeVolumeAdjustmentFrame`](crate::id3::v2::RelativeVolumeAdjustmentFrame) with an invalid channel type
	BadRva2ChannelType,
	/// Arises when decoding a [`TimestampFormat`](crate::id3::v2::TimestampFormat) with an invalid type
	BadTimestampFormat,

	// Compression
	#[cfg(feature = "id3v2_compression_support")]
	/// Arises when a compressed frame is unable to be decompressed
	Decompression(flate2::DecompressError),
	#[cfg(not(feature = "id3v2_compression_support"))]
	/// Arises when a compressed frame is encountered, but support is disabled
	CompressedFrameEncountered,

	// Writing
	/// Arises when attempting to write an encrypted frame with an invalid encryption method symbol (must be <= 0x80)
	InvalidEncryptionMethodSymbol(u8),
	/// Arises when attempting to write an invalid Frame (Bad `FrameId`/`FrameValue` pairing)
	BadFrame(String, &'static str),
	/// Arises when attempting to write a [`CommentFrame`](crate::id3::v2::CommentFrame) or [`UnsynchronizedTextFrame`](crate::id3::v2::UnsynchronizedTextFrame) with an invalid language
	InvalidLanguage([u8; 3]),
}

impl Display for Id3v2ErrorKind {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			// Header
			Self::BadId3v2Version(major, minor) => write!(
				f,
				"Found an invalid version (v{major}.{minor}), expected any major revision in: (2, \
				 3, 4)"
			),
			Self::V2Compression => write!(f, "Encountered a compressed ID3v2.2 tag"),
			Self::BadExtendedHeaderSize => {
				write!(f, "Found an extended header with an invalid size")
			},

			// Frame
			Self::BadFrameId(frame_id) => write!(f, "Failed to parse a frame ID: 0x{frame_id:x?}"),
			Self::UnsupportedFrameId(item_key) => {
				write!(f, "Unsupported frame ID for item key {item_key:?}")
			},
			Self::BadFrameLength => write!(
				f,
				"Frame isn't long enough to extract the necessary information"
			),
			Self::EmptyFrame(id) => write!(f, "Frame `{id}` is empty"),
			Self::MissingDataLengthIndicator => write!(
				f,
				"Encountered an encrypted frame without a data length indicator"
			),
			Self::InvalidUnsynchronisation => write!(f, "Encountered an invalid unsynchronisation"),
			Self::V2InvalidTextEncoding => {
				write!(f, "ID3v2.2 only supports Latin-1 and UTF-16 encodings")
			},
			Self::BadPictureFormat(format) => {
				write!(f, "Picture: Found unexpected format \"{format}\"")
			},
			Self::BadSyncText => write!(f, "Encountered invalid data in SYLT frame"),
			Self::MissingUfidOwner => write!(f, "Missing owner in UFID frame"),
			Self::BadRva2ChannelType => write!(f, "Encountered invalid channel type in RVA2 frame"),
			Self::BadTimestampFormat => write!(
				f,
				"Encountered an invalid timestamp format in a synchronized frame"
			),

			// Compression
			#[cfg(feature = "id3v2_compression_support")]
			Self::Decompression(err) => write!(f, "Failed to decompress frame: {err}"),
			#[cfg(not(feature = "id3v2_compression_support"))]
			Self::CompressedFrameEncountered => write!(
				f,
				"Encountered a compressed ID3v2 frame, support is disabled"
			),

			// Writing
			Self::InvalidEncryptionMethodSymbol(symbol) => write!(
				f,
				"Attempted to write an encrypted frame with an invalid method symbol ({symbol})"
			),
			Self::BadFrame(frame_id, frame_value) => write!(
				f,
				"Attempted to write an invalid frame. ID: \"{frame_id}\", Value: \"{frame_value}\"",
			),
			Self::InvalidLanguage(lang) => write!(
				f,
				"Invalid frame language found: {lang:?} (expected 3 ascii characters)"
			),
		}
	}
}

/// An error that arises while interacting with an ID3v2 tag
pub struct Id3v2Error {
	kind: Id3v2ErrorKind,
}

impl Id3v2Error {
	/// Create a new `ID3v2Error` from an [`Id3v2ErrorKind`]
	#[must_use]
	pub const fn new(kind: Id3v2ErrorKind) -> Self {
		Self { kind }
	}

	/// Returns the [`Id3v2ErrorKind`]
	pub fn kind(&self) -> &Id3v2ErrorKind {
		&self.kind
	}
}

impl Debug for Id3v2Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "ID3v2: {:?}", self.kind)
	}
}

impl Display for Id3v2Error {
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
