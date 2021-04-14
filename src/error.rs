/// Errors that could occur within Lofty.
#[derive(thiserror::Error, Debug)]
pub enum Error {
	/// Unknown file extension.
	#[error("Failed to guess the metadata format based on the file extension.")]
	UnknownFileExtension,

	/// Unsupported file extension
	#[error("Unsupported format: {0}")]
	UnsupportedFormat(String),
	#[error("Unsupported mime type: {0}")]
	UnsupportedMimeType(String),

	#[error(transparent)]
	FlacTag(#[from] metaflac::Error),
	#[error(transparent)]
	Id3Tag(#[from] id3::Error),
	#[error(transparent)]
	Mp4Tag(#[from] mp4ameta::Error),
	#[error(transparent)]
	OpusTag(#[from] opus_headers::ParseError),
	#[error(transparent)]
	Lewton(#[from] lewton::VorbisError),
	#[error(transparent)]
	Ogg(#[from] ogg::OggReadError),

	#[error("")]
	NotAPicture,

	/// Represents all cases of `std::io::Error`.
	#[error(transparent)]
	IO(#[from] std::io::Error),
}

/// Type alias for the result of tag operations.
pub type Result<T> = std::result::Result<T, Error>;
