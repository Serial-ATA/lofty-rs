use crate::error::{TextEncodingError, TooMuchDataError};
use crate::id3::v2::FrameId;
use crate::id3::v2::util::synchsafe::SynchOverflowError;
use crate::tag::items::timestamp::TimestampParseError;
use crate::util::alloc::AllocationError;
use crate::util::text::{BadTextEncodingError, TextDecodingError};

use std::borrow::Cow;

/// Failed to parse an ID3v2 frame
pub struct FrameParseError {
	id: Option<FrameId<'static>>,
	source: Option<Box<dyn core::error::Error + Send + Sync + 'static>>,
}

impl FrameParseError {
	pub(crate) fn new(
		id: Option<FrameId<'static>>,
		source: Box<dyn core::error::Error + Send + Sync + 'static>,
	) -> Self {
		Self {
			id,
			source: Some(source),
		}
	}

	/// Overwrite the frame ID for the error
	pub(crate) fn set_id(&mut self, id: FrameId<'static>) {
		self.id = Some(id);
	}

	/// Wrap an [`std::io::Error`]
	pub(super) fn io(id: Option<FrameId<'static>>, source: std::io::Error) -> Self {
		Self {
			id,
			source: Some(Box::new(source)),
		}
	}

	pub(crate) fn missing_data_length_indicator(id: FrameId<'static>) -> Self {
		Self::message(
			Some(id),
			"encountered an encrypted frame without a data length indicator",
		)
	}

	/// Undersized frame error
	pub(crate) fn undersized(id: FrameId<'static>) -> Self {
		Self::message(
			Some(id),
			"frame isn't long enough to extract the necessary information",
		)
	}

	pub(crate) fn invalid_language(language: [u8; 3]) -> Self {
		Self::message(
			None,
			format!(
				"invalid frame language found: {} (expected 3 ascii characters)",
				language.escape_ascii()
			),
		)
	}

	pub(crate) fn message(
		id: Option<FrameId<'static>>,
		message: impl Into<Cow<'static, str>>,
	) -> Self {
		let message = message.into();
		Self {
			id,
			source: Some(message.into()),
		}
	}
}

impl core::fmt::Debug for FrameParseError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("FrameParseError")
			.field("id", &self.id)
			.finish_non_exhaustive()
	}
}

impl core::fmt::Display for FrameParseError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match &self.id {
			Some(id) => write!(f, "failed to parse frame '{id}'"),
			None => write!(f, "failed to parse a frame"),
		}
	}
}

impl core::error::Error for FrameParseError {
	#[allow(trivial_casts)]
	fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
		self.source.as_ref().map(|e| &**e as _)
	}
}

impl From<std::io::Error> for FrameParseError {
	fn from(input: std::io::Error) -> Self {
		Self {
			id: None,
			source: Some(Box::new(input)),
		}
	}
}

impl From<AllocationError> for FrameParseError {
	fn from(input: AllocationError) -> Self {
		Self {
			id: None,
			source: Some(Box::new(input)),
		}
	}
}

impl From<TextDecodingError> for FrameParseError {
	fn from(input: TextDecodingError) -> Self {
		Self {
			id: None,
			source: Some(Box::new(input)),
		}
	}
}

impl From<BadTextEncodingError> for FrameParseError {
	fn from(input: BadTextEncodingError) -> Self {
		Self {
			id: None,
			source: Some(Box::new(input)),
		}
	}
}

impl From<TimestampParseError> for FrameParseError {
	fn from(input: TimestampParseError) -> Self {
		Self {
			id: None,
			source: Some(Box::new(input)),
		}
	}
}

/// Failed to encode an ID3v2 frame
pub struct FrameEncodingError {
	id: Option<FrameId<'static>>,
	source: Option<Box<dyn core::error::Error + Send + Sync + 'static>>,
}

impl FrameEncodingError {
	pub(crate) fn set_id(&mut self, id: FrameId<'_>) {
		self.id = Some(id.into_owned());
	}

	pub(crate) fn missing_data_length_indicator(id: FrameId<'static>) -> Self {
		Self::message(
			Some(id),
			"encountered an encrypted frame without a data length indicator",
		)
	}

	pub(crate) fn invalid_language(language: [u8; 3]) -> Self {
		Self::message(
			None,
			format!(
				"invalid frame language found: {} (expected 3 ascii characters)",
				language.escape_ascii()
			),
		)
	}

	pub(crate) fn message(
		id: Option<FrameId<'static>>,
		message: impl Into<Cow<'static, str>>,
	) -> Self {
		let message = message.into();
		Self {
			id,
			source: Some(message.into()),
		}
	}
}

impl core::fmt::Debug for FrameEncodingError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("FrameEncodingError")
			.field("id", &self.id)
			.finish_non_exhaustive()
	}
}

impl core::fmt::Display for FrameEncodingError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match &self.id {
			Some(id) => write!(f, "failed to write frame '{id}'"),
			None => write!(f, "failed to write a frame"),
		}
	}
}

impl From<std::io::Error> for FrameEncodingError {
	fn from(input: std::io::Error) -> Self {
		Self {
			id: None,
			source: Some(Box::new(input)),
		}
	}
}

impl From<AllocationError> for FrameEncodingError {
	fn from(input: AllocationError) -> Self {
		Self {
			id: None,
			source: Some(Box::new(input)),
		}
	}
}

impl From<TooMuchDataError> for FrameEncodingError {
	fn from(input: TooMuchDataError) -> Self {
		Self {
			id: None,
			source: Some(Box::new(input)),
		}
	}
}

impl From<TextEncodingError> for FrameEncodingError {
	fn from(input: TextEncodingError) -> Self {
		Self {
			id: None,
			source: Some(Box::new(input)),
		}
	}
}

impl From<TimestampParseError> for FrameEncodingError {
	fn from(input: TimestampParseError) -> Self {
		Self {
			id: None,
			source: Some(Box::new(input)),
		}
	}
}

impl From<SynchOverflowError> for FrameEncodingError {
	fn from(input: SynchOverflowError) -> Self {
		Self {
			id: None,
			source: Some(Box::new(input)),
		}
	}
}

impl core::error::Error for FrameEncodingError {
	#[allow(trivial_casts)]
	fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
		self.source.as_ref().map(|e| &**e as _)
	}
}
