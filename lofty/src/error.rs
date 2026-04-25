//! Contains the errors that can arise within Lofty
//!
//! The primary error is [`struct@LoftyError`]. The type of error is determined by [`ErrorKind`],
//! which can be extended at any time.

use crate::ape::error::ApeTagParseError;
use crate::file::FileType;
use crate::flac::error::FlacParseError;
use crate::id3::Lyrics3v2ParseError;
use crate::id3::v1::error::Id3v1ParseError;
use crate::id3::v2::error::{Id3v2EncodingError, Id3v2ParseError};
use crate::iff::error::ChunkParseError;
use crate::iff::wav::error::WavParseError;
use crate::tag::items::timestamp::TimestampParseError;

use std::error::Error;

use lofty_attr::LoftyError;
use ogg_pager::PageError;

// Exports

pub use crate::util::alloc::AllocationError;
pub use crate::util::text::{TextDecodingError, TextEncodingError};

/// Alias for `Result<T, LoftyError>`
pub type Result<T> = std::result::Result<T, LoftyError>;

/// Failed to parse a file
pub struct FileParseError {
	ty: Option<FileType>,
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl FileParseError {
	pub(crate) fn new(
		ty: FileType,
		source: Box<dyn core::error::Error + Send + Sync + 'static>,
	) -> Self {
		Self {
			ty: Some(ty),
			source,
		}
	}

	/// Whether this error represents an [`UnknownFormatError`]
	pub fn is_unknown_format(&self) -> bool {
		let mut source = self.source();

		while let Some(s) = source {
			if s.is::<UnknownFormatError>() {
				return true;
			}

			source = s.source();
		}

		false
	}
}

impl core::fmt::Display for FileParseError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self.ty {
			Some(ty) => write!(f, "failed to parse {ty:?} file"),
			None => write!(f, "failed to parse file"),
		}
	}
}

impl core::fmt::Debug for FileParseError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("FileParseError")
			.field("ty", &self.ty)
			.finish_non_exhaustive()
	}
}

impl core::error::Error for FileParseError {
	fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
		Some(self.source.as_ref())
	}
}

impl From<std::io::Error> for FileParseError {
	fn from(input: std::io::Error) -> Self {
		Self {
			ty: None,
			source: Box::new(input),
		}
	}
}

impl From<UnknownFormatError> for FileParseError {
	fn from(input: UnknownFormatError) -> Self {
		Self {
			ty: None,
			source: Box::new(input),
		}
	}
}

// TODO: remove this
impl From<LoftyError> for FileParseError {
	fn from(input: LoftyError) -> Self {
		Self {
			ty: None,
			source: Box::new(input),
		}
	}
}

/// Arises when a tag is expected (Ex. found an "ID3 " chunk in a WAV file), but isn’t found
#[derive(LoftyError)]
#[error(message = "expected a tag, found invalid data")]
pub struct FakeTagError;

/// Attempting to read/write an abnormally large amount of data
#[derive(LoftyError)]
#[error(message = "attempted to read/write an abnormally large amount of data")]
pub struct TooMuchDataError;

// TODO: Should definitely have a mandatory context message
/// Expected the data to be a different size than provided
///
/// This occurs when the size of an item is written as one value, but that size is either too
/// big or small to be valid within the bounds of that item.
#[derive(LoftyError)]
#[error(message = "encountered an invalid item size, either too big or too small to be valid")]
pub struct SizeMismatchError;

/// Unable to guess the format of the input
#[derive(LoftyError)]
#[error(message = "no format could be determined from the provided file")]
pub struct UnknownFormatError;

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
	// TODO: Remove this and `FileDecoding`
	/// Errors that occur while decoding a file
	FileParse(FileParseError),
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
	TextDecode(TextDecodingError),
	/// Errors that arise while encoding text
	TextEncode(TextEncodingError),
	/// Arises when decoding OR encoding a problematic [`Timestamp`](crate::tag::items::Timestamp)
	BadTimestamp(TimestampParseError),
	// TODO: remove these
	/// Errors that can occur while parsing tags
	TagParse,
	/// Errors that can occur while encoding tags
	TagEncoding,

	/// Arises when an atom contains invalid data
	BadAtom(&'static str),
	/// Arises when attempting to use [`Atom::merge`](crate::mp4::Atom::merge) with mismatching identifiers
	AtomMismatch,

	// Conversions for external errors
	/// Errors that arise while parsing OGG pages
	OggPage(ogg_pager::PageError),
	/// Represents all cases of [`std::io::Error`].
	Io(std::io::Error),
	/// Represents all cases of [`std::fmt::Error`].
	Fmt(std::fmt::Error),
	/// Failure to allocate enough memory
	Alloc(AllocationError),
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

impl core::fmt::Debug for FileDecodingError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> std::fmt::Result {
		if let Some(format) = self.format {
			write!(f, "{:?}: {:?}", format, self.description)
		} else {
			write!(f, "{:?}", self.description)
		}
	}
}

impl core::fmt::Display for FileDecodingError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> std::fmt::Result {
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

impl core::fmt::Debug for FileEncodingError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> std::fmt::Result {
		if let Some(format) = self.format {
			write!(f, "{:?}: {:?}", format, self.description)
		} else {
			write!(f, "{:?}", self.description)
		}
	}
}

impl core::fmt::Display for FileEncodingError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> std::fmt::Result {
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
	source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
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
		Self { kind, source: None }
	}

	/// Create a `LoftyError` with a source error
	pub fn with_source<E>(kind: ErrorKind, source: E) -> Self
	where
		E: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
	{
		Self {
			kind,
			source: Some(source.into()),
		}
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

impl std::error::Error for LoftyError {
	#[allow(trivial_casts)]
	fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
		self.source.as_ref().map(|e| &**e as _)
	}
}

impl core::fmt::Debug for LoftyError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:?}", self.kind)
	}
}

impl From<TimestampParseError> for LoftyError {
	fn from(input: TimestampParseError) -> Self {
		Self::new(ErrorKind::BadTimestamp(input))
	}
}

impl From<FileDecodingError> for LoftyError {
	fn from(input: FileDecodingError) -> Self {
		Self::new(ErrorKind::FileDecoding(input))
	}
}

impl From<FileEncodingError> for LoftyError {
	fn from(input: FileEncodingError) -> Self {
		Self::new(ErrorKind::FileEncoding(input))
	}
}

impl From<TextEncodingError> for LoftyError {
	fn from(input: TextEncodingError) -> Self {
		Self::new(ErrorKind::TextEncode(input))
	}
}

impl From<TextDecodingError> for LoftyError {
	fn from(input: TextDecodingError) -> Self {
		Self::new(ErrorKind::TextDecode(input))
	}
}

impl From<ogg_pager::PageError> for LoftyError {
	fn from(input: PageError) -> Self {
		Self::new(ErrorKind::OggPage(input))
	}
}

impl From<std::io::Error> for LoftyError {
	fn from(input: std::io::Error) -> Self {
		Self::new(ErrorKind::Io(input))
	}
}

impl From<std::fmt::Error> for LoftyError {
	fn from(input: std::fmt::Error) -> Self {
		Self::new(ErrorKind::Fmt(input))
	}
}

impl From<std::string::FromUtf8Error> for LoftyError {
	fn from(input: std::string::FromUtf8Error) -> Self {
		Self::new(ErrorKind::TextDecode(input.into()))
	}
}

impl From<std::str::Utf8Error> for LoftyError {
	fn from(input: std::str::Utf8Error) -> Self {
		Self::new(ErrorKind::TextDecode(input.into()))
	}
}

impl From<AllocationError> for LoftyError {
	fn from(input: AllocationError) -> Self {
		Self::new(ErrorKind::Alloc(input))
	}
}

impl From<std::convert::Infallible> for LoftyError {
	fn from(input: std::convert::Infallible) -> Self {
		Self::new(ErrorKind::Infallible(input))
	}
}

// TODO: Remove this
impl From<Id3v2EncodingError> for LoftyError {
	fn from(_: Id3v2EncodingError) -> Self {
		Self::new(ErrorKind::TagEncoding)
	}
}

// TODO: Remove this
impl From<FakeTagError> for LoftyError {
	fn from(_: FakeTagError) -> Self {
		Self::new(ErrorKind::FakeTag)
	}
}

// TODO: Remove this
impl From<TooMuchDataError> for LoftyError {
	fn from(_: TooMuchDataError) -> Self {
		Self::new(ErrorKind::TooMuchData)
	}
}

// TODO: Remove this
impl From<UnknownFormatError> for LoftyError {
	fn from(_: UnknownFormatError) -> Self {
		Self::new(ErrorKind::UnknownFormat)
	}
}

// TODO: Remove this
impl From<Lyrics3v2ParseError> for LoftyError {
	fn from(_: Lyrics3v2ParseError) -> Self {
		Self::new(ErrorKind::TagParse)
	}
}

// TODO: Remove this
impl From<Id3v1ParseError> for LoftyError {
	fn from(_: Id3v1ParseError) -> Self {
		Self::new(ErrorKind::TagParse)
	}
}

// TODO: Remove this
impl From<Id3v2ParseError> for LoftyError {
	fn from(_: Id3v2ParseError) -> Self {
		Self::new(ErrorKind::TagParse)
	}
}

// TODO: Remove this
impl From<ApeTagParseError> for LoftyError {
	fn from(_: ApeTagParseError) -> Self {
		Self::new(ErrorKind::TagParse)
	}
}

// TODO: Remove this
impl From<ChunkParseError> for LoftyError {
	fn from(_: ChunkParseError) -> Self {
		Self::new(ErrorKind::TagParse)
	}
}

// TODO: Remove this
impl From<WavParseError> for LoftyError {
	fn from(input: WavParseError) -> Self {
		Self::new(ErrorKind::FileParse(input.into()))
	}
}

// TODO: Remove this
impl From<FlacParseError> for LoftyError {
	fn from(input: FlacParseError) -> Self {
		Self::new(ErrorKind::FileParse(input.into()))
	}
}

impl From<FileParseError> for LoftyError {
	fn from(input: FileParseError) -> Self {
		Self::new(ErrorKind::FileParse(input))
	}
}

impl core::fmt::Display for LoftyError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> std::fmt::Result {
		match &self.kind {
			// Conversions
			ErrorKind::OggPage(err) => write!(f, "{err}"),
			ErrorKind::Io(err) => write!(f, "{err}"),
			ErrorKind::Fmt(err) => write!(f, "{err}"),
			ErrorKind::Alloc(err) => write!(f, "{err}"),

			ErrorKind::UnknownFormat => {
				write!(f, "{}", UnknownFormatError)
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
			ErrorKind::TagParse => write!(f, "failed to parse tag"),
			ErrorKind::TagEncoding => write!(f, "failed to encode tag"),
			ErrorKind::BadAtom(message) => write!(f, "MP4 Atom: {message}"),
			ErrorKind::AtomMismatch => write!(
				f,
				"MP4 Atom: Attempted to use `Atom::merge()` with mismatching identifiers"
			),

			// Files
			ErrorKind::TooMuchData => write!(f, "{}", TooMuchDataError),
			ErrorKind::SizeMismatch => write!(f, "{}", SizeMismatchError),
			ErrorKind::FileParse(e) => write!(f, "{e}"),
			ErrorKind::FileDecoding(file_decode_err) => write!(f, "{file_decode_err}"),
			ErrorKind::FileEncoding(file_encode_err) => write!(f, "{file_encode_err}"),

			ErrorKind::Infallible(_) => write!(f, "A expected condition was not upheld"),
		}
	}
}
