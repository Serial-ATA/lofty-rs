//! ID3v2 error types

use crate::id3::v2::{FrameId, Id3v2Version};
use crate::prelude::ItemKey;

use std::fmt::{Debug, Display, Formatter};

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
	/// Arises when a frame or tag has its unsynchronisation flag set, but the content is not actually syncsafe
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
	/// Arises when attempting to read/write a frame in a version that doesn't support it
	UnsupportedVersion {
		/// The ID of the frame being read/written
		id: FrameId<'static>,
		/// The version provided
		version: Id3v2Version,
	},

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
			Self::BadFrameId(frame_id) => {
				write!(f, "Failed to parse a frame ID: {}", frame_id.escape_ascii())
			},
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
			Self::UnsupportedVersion { id, version } => write!(
				f,
				"attempted to read/write '{}' frame in version {version}",
				id.as_str()
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
