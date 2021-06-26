/// Errors that could occur within Lofty.
#[derive(thiserror::Error, Debug)]
pub enum LoftyError {
	// File extension/format related errors
	/// Unknown file extension.
	#[error("Failed to guess the metadata format based on the file extension.")]
	UnknownFileExtension,
	/// Unsupported file extension
	#[error("Unsupported format: {0}")]
	UnsupportedFormat(String),
	/// Unable to guess the format
	#[error("No format could be determined from the provided file.")]
	UnknownFormat,

	// File data related errors
	/// Provided an empty file
	#[error("File contains no data")]
	EmptyFile,
	/// Provided a file with invalid/malformed data
	#[error("File has invalid data: {0}")]
	InvalidData(&'static str),
	/// Attempting to write an abnormally large amount of data
	#[error("An abnormally large amount of data was provided, and an overflow occurred")]
	TooMuchData,

	// Picture related errors
	/// Picture has an unsupported mime type
	#[error("Unsupported mime type: {0}")]
	UnsupportedMimeType(String),
	/// Provided an invalid picture
	#[error("Picture contains invalid data")]
	NotAPicture,

	// Tag related errors
	/// Any error from [`ape`]
	#[error(transparent)]
	ApeTag(#[from] ape::Error),
	/// Any error from [`metaflac`]
	#[error(transparent)]
	FlacTag(#[from] metaflac::Error),
	/// Any error from [`id3`]
	#[error(transparent)]
	Id3Tag(#[from] id3::Error),
	/// Any error from [`mp3_duration`]
	#[cfg(feature = "duration")]
	#[error(transparent)]
	Mp3Duration(#[from] mp3_duration::MP3DurationError),
	/// Any error from [`mp4ameta`]
	#[error(transparent)]
	Mp4Tag(#[from] mp4ameta::Error),
	/// Any error from [`lewton`]
	#[error(transparent)]
	Lewton(#[from] lewton::VorbisError),
	/// Any error from [`ogg`]
	#[error(transparent)]
	Ogg(#[from] ogg::OggReadError),
	/// Errors that arrist while parsing OGG pages
	#[error(transparent)]
	OggPage(#[from] ogg_pager::PageError),
	/// Errors that arise while reading/writing to wav files
	#[error("Invalid Riff file: {0}")]
	Riff(&'static str),

	// Conversions for std Errors
	/// Unable to convert bytes to a String
	#[error(transparent)]
	FromUtf8(#[from] std::string::FromUtf8Error),
	/// Represents all cases of `std::io::Error`.
	#[error(transparent)]
	Io(#[from] std::io::Error),
}

/// Result of tag operations.
pub type Result<T> = std::result::Result<T, LoftyError>;
