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
	/// Attempting to write an abnormally large amount of data
	#[error("An abnormally large amount of data was provided, and an overflow occurred")]
	TooMuchData,

	// Picture related errors
	/// Arises when an invalid picture format is parsed. Only applicable to [`Id3v2Version::V2`](crate::logic::id3::v2::Id3v2Version)
	#[error("Picture: Found unexpected format {0}")]
	BadPictureFormat(String),
	/// Provided an invalid picture
	#[error("Picture: Encountered invalid data")]
	NotAPicture,

	// Tag related errors
	#[cfg(feature = "id3")] // TODO
	/// Any error from [`id3`]
	#[error(transparent)]
	Id3Tag(#[from] id3::Error),
	#[cfg(feature = "mp4ameta")] // TODO
	/// Any error from [`mp4ameta`]
	#[error(transparent)]
	Mp4Tag(#[from] mp4ameta::Error),
	/// Errors that arise while parsing OGG pages
	#[cfg(feature = "vorbis_comments")]
	#[error(transparent)]
	OggPage(#[from] ogg_pager::PageError),
	/// Errors that arise while decoding ID3v2 text
	#[error("Text decoding: {0}")]
	TextDecode(&'static str),
	/// Arises when invalid data is encountered while reading an ID3v2 synchronized text frame
	#[error("ID3v2: Encountered invalid data in SYLT frame")]
	BadSyncText,
	/// Errors that arise while reading/writing to WAV files
	#[error("Riff: {0}")]
	Wav(&'static str),
	/// Errors that arise while reading/writing to AIFF files
	#[error("Aiff: {0}")]
	Aiff(&'static str),
	/// Errors that arise while reading/writing to FLAC files
	#[error("Flac: {0}")]
	Flac(&'static str),
	/// Errors that arise while reading/writing to OPUS files
	#[error("Opus: {0}")]
	Opus(&'static str),
	/// Errors that arise while reading/writing to OGG Vorbis files
	#[error("Vorbis: {0}")]
	Vorbis(&'static str),
	/// Errors that arise while reading/writing to OGG files
	#[error("OGG: {0}")]
	Ogg(&'static str),
	/// Errors that arise while reading/writing to MPEG files
	#[error("MPEG: {0}")]
	Mpeg(&'static str),
	/// Errors that arise while reading/writing to APE files
	#[error("APE: {0}")]
	Ape(&'static str),

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
