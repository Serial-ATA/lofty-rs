//! Contains the errors that can arise within Lofty
//!
//! The primary error is [`struct@LoftyError`]. The type of error is determined by [`ErrorKind`],
//! which can be extended at any time.

use crate::file::FileType;
use crate::flac::error::FlacParseError;
use crate::id3::Lyrics3v2ParseError;
use crate::iff::aiff::error::AiffParseError;
use crate::iff::error::ChunkParseError;
use crate::iff::wav::error::WavParseError;
use crate::mp4::error::{AtomParseError, Mp4ParseError};
use crate::tag::TagType;
use crate::tag::items::timestamp::TimestampParseError;

use std::error::Error;

use lofty_attr::LoftyError;
use ogg_pager::PageError;

// Exports

pub use crate::util::alloc::AllocationError;
pub use crate::util::text::{TextDecodingError, TextEncodingError};

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

	pub(crate) fn with_format(mut self, format: FileType) -> Self {
		self.ty = Some(format);
		self
	}

	pub(crate) fn message(ty: Option<FileType>, message: &'static str) -> Self {
		Self {
			ty,
			source: message.into(),
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

impl From<ogg_pager::PageError> for FileParseError {
	fn from(input: ogg_pager::PageError) -> Self {
		Self {
			ty: None,
			source: Box::new(input),
		}
	}
}

impl From<TagParseError> for FileParseError {
	fn from(input: TagParseError) -> Self {
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

impl From<SizeMismatchError> for FileParseError {
	fn from(input: SizeMismatchError) -> Self {
		Self {
			ty: None,
			source: Box::new(input),
		}
	}
}

impl From<Lyrics3v2ParseError> for FileParseError {
	fn from(input: Lyrics3v2ParseError) -> Self {
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

/// An error that arises while encoding a file
pub struct FileEncodingError {
	format: Option<FileType>,
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl FileEncodingError {
	pub(crate) fn new(
		format: FileType,
		source: Box<dyn core::error::Error + Send + Sync + 'static>,
	) -> Self {
		Self {
			format: Some(format),
			source,
		}
	}

	pub(crate) fn with_format(mut self, format: FileType) -> Self {
		self.format = Some(format);
		self
	}
}

impl core::fmt::Display for FileEncodingError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self.format {
			Some(format) => write!(f, "failed to write {format:?} file"),
			None => write!(f, "failed to write to file"),
		}
	}
}

impl core::fmt::Debug for FileEncodingError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("FileEncodingError")
			.field("format", &self.format)
			.finish_non_exhaustive()
	}
}

impl core::error::Error for FileEncodingError {
	fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
		Some(self.source.as_ref())
	}
}

impl From<core::convert::Infallible> for FileEncodingError {
	fn from(_: core::convert::Infallible) -> Self {
		unreachable!()
	}
}

impl From<std::io::Error> for FileEncodingError {
	fn from(input: std::io::Error) -> Self {
		Self {
			format: None,
			source: Box::new(input),
		}
	}
}

impl From<TagEncodingError> for FileEncodingError {
	fn from(input: TagEncodingError) -> Self {
		Self {
			format: None,
			source: Box::new(input),
		}
	}
}

impl From<UnknownFormatError> for FileEncodingError {
	fn from(input: UnknownFormatError) -> Self {
		Self {
			format: None,
			source: Box::new(input),
		}
	}
}

impl From<UnsupportedTagError> for FileEncodingError {
	fn from(input: UnsupportedTagError) -> Self {
		Self {
			format: None,
			source: Box::new(input),
		}
	}
}

impl From<FileParseError> for FileEncodingError {
	fn from(input: FileParseError) -> Self {
		Self {
			format: input.ty,
			source: Box::new(input),
		}
	}
}

impl From<TagParseError> for FileEncodingError {
	fn from(input: TagParseError) -> Self {
		Self {
			format: None,
			source: Box::new(input),
		}
	}
}

impl From<SizeMismatchError> for FileEncodingError {
	fn from(input: SizeMismatchError) -> Self {
		Self {
			format: None,
			source: Box::new(input),
		}
	}
}

impl From<TooMuchDataError> for FileEncodingError {
	fn from(input: TooMuchDataError) -> Self {
		Self {
			format: None,
			source: Box::new(input),
		}
	}
}

impl From<AllocationError> for FileEncodingError {
	fn from(input: AllocationError) -> Self {
		Self {
			format: None,
			source: Box::new(input),
		}
	}
}

// TODO: remove this
impl From<LoftyError> for FileEncodingError {
	fn from(input: LoftyError) -> Self {
		Self {
			format: None,
			source: Box::new(input),
		}
	}
}

/// An error that arises while parsing a tag
pub struct TagParseError {
	ty: Option<TagType>,
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl TagParseError {
	pub(crate) fn new(
		ty: TagType,
		source: Box<dyn core::error::Error + Send + Sync + 'static>,
	) -> Self {
		Self {
			ty: Some(ty),
			source,
		}
	}
}

impl core::fmt::Display for TagParseError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self.ty {
			Some(format) => write!(f, "failed to parse {format:?} tag"),
			None => write!(f, "failed to parse tag"),
		}
	}
}

impl core::fmt::Debug for TagParseError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("TagParseError")
			.field("ty", &self.ty)
			.finish_non_exhaustive()
	}
}

impl core::error::Error for TagParseError {
	fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
		Some(self.source.as_ref())
	}
}

/// An error that arises while encoding a tag
pub struct TagEncodingError {
	ty: Option<TagType>,
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl TagEncodingError {
	pub(crate) fn new(
		ty: TagType,
		source: Box<dyn core::error::Error + Send + Sync + 'static>,
	) -> Self {
		Self {
			ty: Some(ty),
			source,
		}
	}
}

impl core::fmt::Display for TagEncodingError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self.ty {
			Some(format) => write!(f, "failed to write {format:?} tag"),
			None => write!(f, "failed to write tag"),
		}
	}
}

impl core::fmt::Debug for TagEncodingError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("TagEncodingError")
			.field("ty", &self.ty)
			.finish_non_exhaustive()
	}
}

impl core::error::Error for TagEncodingError {
	fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
		Some(self.source.as_ref())
	}
}

// TODO: remove this
impl From<LoftyError> for TagEncodingError {
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

/// Attempting to parse an item, but there isn't enough data available
#[derive(Debug)]
pub struct NotEnoughDataError {
	expected: Option<usize>,
}

impl NotEnoughDataError {
	pub(crate) fn new(expected: Option<usize>) -> Self {
		Self { expected }
	}
}

impl core::fmt::Display for NotEnoughDataError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self.expected {
			None => write!(f, "not enough data in reader"),
			Some(expected) => write!(f, "not enough data in reader (expected {expected} bytes)"),
		}
	}
}

impl core::error::Error for NotEnoughDataError {}

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

/// Attempting to write a tag to a [`FileType`] that doesn't support it
#[derive(LoftyError)]
#[error(message = "attempted to write a tag to a format that does not support it")]
pub struct UnsupportedTagError;

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
	/// Errors that occur while encoding a file
	FileEncoding(FileEncodingError),

	// Picture related errors
	/// Provided an invalid picture
	NotAPicture,
	/// Attempted to write a picture that the format does not support
	UnsupportedPicture,

	// Tag related errors
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
impl From<AiffParseError> for LoftyError {
	fn from(input: AiffParseError) -> Self {
		Self::new(ErrorKind::FileParse(input.into()))
	}
}

// TODO: Remove this
impl From<FlacParseError> for LoftyError {
	fn from(input: FlacParseError) -> Self {
		Self::new(ErrorKind::FileParse(input.into()))
	}
}

// TODO: Remove this
impl From<Mp4ParseError> for LoftyError {
	fn from(input: Mp4ParseError) -> Self {
		Self::new(ErrorKind::FileParse(input.into()))
	}
}

// TODO: Remove this
impl From<AtomParseError> for LoftyError {
	fn from(_: AtomParseError) -> Self {
		Self::new(ErrorKind::TagParse)
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
			ErrorKind::FakeTag => write!(f, "Reading: Expected a tag, found invalid data"),
			ErrorKind::TextDecode(message) => write!(f, "Text decoding: {message}"),
			ErrorKind::TextEncode(message) => write!(f, "Text encoding: {message}"),
			ErrorKind::BadTimestamp(message) => {
				write!(f, "Encountered an invalid timestamp: {message}")
			},
			ErrorKind::TagParse => write!(f, "failed to parse tag"),
			ErrorKind::TagEncoding => write!(f, "failed to encode tag"),
			ErrorKind::AtomMismatch => write!(
				f,
				"MP4 Atom: Attempted to use `Atom::merge()` with mismatching identifiers"
			),

			// Files
			ErrorKind::TooMuchData => write!(f, "{}", TooMuchDataError),
			ErrorKind::SizeMismatch => write!(f, "{}", SizeMismatchError),
			ErrorKind::FileParse(e) => write!(f, "{e}"),
			ErrorKind::FileEncoding(file_encode_err) => write!(f, "{file_encode_err}"),

			ErrorKind::Infallible(_) => write!(f, "A expected condition was not upheld"),
		}
	}
}
