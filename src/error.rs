/// Errors that could occur within Lofty.
#[derive(thiserror::Error, Debug)]
pub enum Error {
	/// Unknown file extension.
	#[error("Failed to guess the metadata format based on the file extension.")]
	UnknownFileExtension,

	#[error("No format could be determined from the provided file.")]
	UnknownFormat,
	#[error("File contains no data")]
	EmptyFile,

	/// Unsupported file extension
	#[error("Unsupported format: {0}")]
	UnsupportedFormat(String),
	#[error("Unsupported mime type: {0}")]
	UnsupportedMimeType(String),

	#[error(transparent)]
	ApeTag(#[from] ape::Error),
	#[error(transparent)]
	FlacTag(#[from] metaflac::Error),
	#[error(transparent)]
	Id3Tag(#[from] id3::Error),
	#[cfg(feature = "duration")]
	#[error(transparent)]
	MP3Duration(#[from] mp3_duration::MP3DurationError),
	#[error(transparent)]
	Mp4Tag(#[from] mp4ameta::Error),
	#[error(transparent)]
	OpusTag(#[from] opus_headers::ParseError),
	#[error(transparent)]
	Lewton(#[from] lewton::VorbisError),
	#[error(transparent)]
	Ogg(#[from] ogg::OggReadError),
	#[error("{0}")]
	Wav(String),

	#[error("")]
	NotAPicture,

	#[error(transparent)]
	Utf8(#[from] std::str::Utf8Error),
	#[error(transparent)]
	FromUtf8(#[from] std::string::FromUtf8Error),
	/// Represents all cases of `std::io::Error`.
	#[error(transparent)]
	IO(#[from] std::io::Error),
}

/// Type alias for the result of tag operations.
pub type Result<T> = std::result::Result<T, Error>;
