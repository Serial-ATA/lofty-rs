//! ID3v2 error types

use crate::prelude::ItemKey;

use lofty_attr::LoftyError;

// Exports

pub use super::frame::error::{FrameEncodingError, FrameParseError};
pub use super::frame::header::FrameIdParseError;
pub use super::items::sync_text::{BadSyncTextContentTypeError, BadTimestampFormatError};

/// The types of errors that can occur while interacting with ID3v2 tags
#[derive(Debug)]
#[non_exhaustive]
pub enum Id3v2ErrorKind {
	// Frame
	/// Arises when no frame ID is available in the ID3v2 specification for an item key
	/// and the associated value type.
	UnsupportedFrameId(ItemKey),
	/// Arises when a frame or tag has its unsynchronisation flag set, but the content is not actually synchsafe
	///
	/// See [`FrameFlags::unsynchronisation`](crate::id3::v2::FrameFlags::unsynchronisation) for an explanation.
	InvalidUnsynchronisation,
	/// Arises when a text encoding other than Latin-1 or UTF-16 appear in an ID3v2.2 tag
	V2InvalidTextEncoding,
	/// Arises when invalid data is encountered while reading an ID3v2 synchronized text frame
	BadSyncText,
	/// Arises when decoding a [`UniqueFileIdentifierFrame`](crate::id3::v2::UniqueFileIdentifierFrame) with no owner
	MissingUfidOwner,
	/// Arises when decoding a [`TimestampFormat`](crate::id3::v2::TimestampFormat) with an invalid type
	BadTimestampFormat,
}

impl core::fmt::Display for Id3v2ErrorKind {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			// Frame
			Self::UnsupportedFrameId(item_key) => {
				write!(f, "Unsupported frame ID for item key {item_key:?}")
			},
			Self::InvalidUnsynchronisation => write!(f, "Encountered an invalid unsynchronisation"),
			Self::V2InvalidTextEncoding => {
				write!(f, "ID3v2.2 only supports Latin-1 and UTF-16 encodings")
			},
			Self::BadSyncText => write!(f, "Encountered invalid data in SYLT frame"),
			Self::MissingUfidOwner => write!(f, "Missing owner in UFID frame"),
			Self::BadTimestampFormat => write!(
				f,
				"Encountered an invalid timestamp format in a synchronized frame"
			),
		}
	}
}

/// Errors that can occur while parsing ID3v2 tag headers
#[derive(Debug)]
pub enum Id3v2HeaderError {
	/// Arises when an invalid ID3v2 version is found
	BadId3v2Version(u8, u8),
	/// Arises when a compressed ID3v2.2 tag is encountered
	///
	/// At the time the ID3v2.2 specification was written, a compression scheme wasn't decided.
	/// As such, it is recommended to ignore the tag entirely.
	V2Compression,
	/// Arises when an extended header has an invalid size (must be >= 6 bytes and less than the total tag size)
	BadExtendedHeaderSize,
}

impl core::fmt::Display for Id3v2HeaderError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Id3v2HeaderError::BadId3v2Version(major, minor) => write!(
				f,
				"found an invalid version (v{major}.{minor}), expected any major revision in: (2, \
				 3, 4)"
			),
			Id3v2HeaderError::V2Compression => write!(f, "encountered a compressed ID3v2.2 tag"),
			Id3v2HeaderError::BadExtendedHeaderSize => {
				write!(f, "encountered an extended header with an invalid size")
			},
		}
	}
}

impl core::error::Error for Id3v2HeaderError {}

/// Errors that can occur while parsing an ID3v2 tag
#[derive(LoftyError)]
#[error(message = "failed to parse ID3v2 tag")]
pub struct Id3v2ParseError {
	#[error(from(
		std::io::Error,
		crate::error::FakeTagError,
		Id3v2HeaderError,
		super::frame::error::FrameParseError,
		crate::util::alloc::AllocationError,
		crate::iff::error::ChunkParseError,
	))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

/// Errors that can occur while encoding an ID3v2 tag
#[derive(LoftyError)]
#[error(message = "failed to write ID3v2 tag")]
pub struct Id3v2EncodingError {
	#[error(from(
		std::io::Error,
		super::frame::error::FrameEncodingError,
		crate::id3::v2::util::synchsafe::SynchOverflowError,
		crate::util::alloc::AllocationError,
	))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}
