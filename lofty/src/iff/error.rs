//! IFF error types

use crate::error::{AllocationError, SizeMismatchError, TextDecodingError};

/// Failed to parse a chunk
pub struct ChunkParseError {
	fourcc: Option<[u8; 4]>,
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl ChunkParseError {
	pub(super) fn with_fourcc(mut self, fourcc: [u8; 4]) -> Self {
		self.fourcc = Some(fourcc);
		self
	}

	/// Whether the chunk failed to parse due to a [`TextDecodingError`]
	pub fn is_text_decoding_error(&self) -> bool {
		self.source.is::<TextDecodingError>()
	}
}

impl core::fmt::Display for ChunkParseError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self.fourcc {
			Some(fourcc) => write!(f, "failed to parse chunk '{}'", fourcc.escape_ascii()),
			None => write!(f, "failed to parse chunk"),
		}
	}
}

impl core::fmt::Debug for ChunkParseError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("ChunkParseError").finish_non_exhaustive()
	}
}

impl core::error::Error for ChunkParseError {
	fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
		Some(&*self.source)
	}
}

impl From<std::io::Error> for ChunkParseError {
	fn from(input: std::io::Error) -> Self {
		Self {
			fourcc: None,
			source: Box::new(input),
		}
	}
}

impl From<SizeMismatchError> for ChunkParseError {
	fn from(input: SizeMismatchError) -> Self {
		Self {
			fourcc: None,
			source: Box::new(input),
		}
	}
}

impl From<AllocationError> for ChunkParseError {
	fn from(input: AllocationError) -> Self {
		Self {
			fourcc: None,
			source: Box::new(input),
		}
	}
}

impl From<TextDecodingError> for ChunkParseError {
	fn from(input: TextDecodingError) -> Self {
		Self {
			fourcc: None,
			source: Box::new(input),
		}
	}
}
