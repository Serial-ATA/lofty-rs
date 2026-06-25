//! Contains the errors that can arise within Lofty

use crate::file::FileType;
use crate::id3::Lyrics3v2ParseError;
use crate::tag::TagType;

use std::error::Error;

use lofty_attr::LoftyError;

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
