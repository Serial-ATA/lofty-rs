/// Errors that could occur within Lofty.
#[derive(thiserror::Error, Debug)]
pub enum Error {
	/// Unknown file extension.
	#[error("Failed to guess the metadata format based on the file extension.")]
	UnknownFileExtension(String),

	/// Unsupported file extension
	#[error("Unsupported format: {0}")]
	UnsupportedFormat(String),
	#[error("Unsupported mime type: {0}")]
	UnsupportedMimeType(String),

	#[error(transparent)]
	FlacTagError(#[from] claxon::Error),
	#[error(transparent)]
	Id3TagError(#[from] id3::Error),
	#[error(transparent)]
	Mp4TagError(#[from] mp4ameta::Error),
	#[error(transparent)]
	OpusTagError(#[from] opus_headers::ParseError),
	#[error(transparent)]
	OggTagError(#[from] lewton::VorbisError),

	#[error("")]
	NotAPicture,

	/// Represents all cases of `std::io::Error`.
	#[error(transparent)]
	IOError(#[from] std::io::Error),
}

/// Type alias for the result of tag operations.
pub type Result<T> = std::result::Result<T, Error>;
