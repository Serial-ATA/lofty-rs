//! MP4 file/tag error types

use crate::error::{
	AllocationError, FileParseError, NotEnoughDataError, SizeMismatchError, TextDecodingError,
	TooMuchDataError,
};
use crate::file::FileType;
use crate::mp4::AtomIdent;

use std::borrow::Cow;

use lofty_attr::LoftyError;

// Exports

pub use super::ilst::error::IlstParseError;

/// Failed to parse a [`Mp4File`]
///
/// [`Mp4File`]: crate::mp4::Mp4File
#[derive(LoftyError)]
#[error(message = "failed to parse MP4 file")]
pub struct Mp4ParseError {
	#[error(from(
		std::io::Error,
		IlstParseError,
		AtomParseError,
		crate::error::UnknownFormatError,
		crate::error::NotEnoughDataError,
	))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl Mp4ParseError {
	pub(super) fn message(message: &'static str) -> Self {
		Self {
			source: message.into(),
		}
	}
}

impl From<Mp4ParseError> for FileParseError {
	fn from(input: Mp4ParseError) -> FileParseError {
		Self::new(FileType::Mp4, input.source)
	}
}

/// Failed to parse an atom
pub struct AtomParseError {
	ident: Option<AtomIdent<'static>>,
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl AtomParseError {
	pub(super) fn with_ident(mut self, ident: AtomIdent<'_>) -> Self {
		self.ident = Some(ident.into_owned());
		self
	}

	/// Set the ident for this error, only if one isn't already set
	pub(super) fn with_ident_if_not_present(self, ident: AtomIdent<'_>) -> Self {
		if self.ident.is_some() {
			return self;
		}

		self.with_ident(ident)
	}

	pub(super) fn message(
		ident: Option<AtomIdent<'_>>,
		message: impl Into<Cow<'static, str>>,
	) -> Self {
		let message = message.into();
		Self {
			ident: ident.map(AtomIdent::into_owned),
			source: message.into(),
		}
	}
}

impl core::fmt::Display for AtomParseError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match &self.ident {
			Some(ident) => write!(f, "failed to parse atom '{ident}'"),
			None => write!(f, "failed to parse atom"),
		}
	}
}

impl core::fmt::Debug for AtomParseError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("AtomParseError").finish_non_exhaustive()
	}
}

impl core::error::Error for AtomParseError {
	fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
		Some(&*self.source)
	}
}

impl From<std::io::Error> for AtomParseError {
	fn from(input: std::io::Error) -> Self {
		Self {
			ident: None,
			source: Box::new(input),
		}
	}
}

impl From<SizeMismatchError> for AtomParseError {
	fn from(input: SizeMismatchError) -> Self {
		Self {
			ident: None,
			source: Box::new(input),
		}
	}
}

impl From<TooMuchDataError> for AtomParseError {
	fn from(input: TooMuchDataError) -> Self {
		Self {
			ident: None,
			source: Box::new(input),
		}
	}
}

impl From<AllocationError> for AtomParseError {
	fn from(input: AllocationError) -> Self {
		Self {
			ident: None,
			source: Box::new(input),
		}
	}
}

impl From<NotEnoughDataError> for AtomParseError {
	fn from(input: NotEnoughDataError) -> Self {
		Self {
			ident: None,
			source: Box::new(input),
		}
	}
}

impl From<TextDecodingError> for AtomParseError {
	fn from(input: TextDecodingError) -> Self {
		Self {
			ident: None,
			source: Box::new(input),
		}
	}
}

// TODO: Remove this
impl From<crate::error::LoftyError> for AtomParseError {
	fn from(input: crate::error::LoftyError) -> Self {
		Self {
			ident: None,
			source: Box::new(input),
		}
	}
}
