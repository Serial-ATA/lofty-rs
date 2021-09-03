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
	/// Attempting to read/write an abnormally large amount of data
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
	/// Arises when writing a tag to a file type that doesn't support it
	#[error("Attempted to write a tag to a format that does not support it")]
	UnsupportedTag,
	/// Errors that arise while parsing OGG pages
	#[cfg(feature = "vorbis_comments")]
	#[error(transparent)]
	OggPage(#[from] ogg_pager::PageError),
	/// Errors that arise while decoding ID3v2 text
	#[error("Text decoding: {0}")]
	TextDecode(&'static str),
	/// Errors that arise while reading/writing ID3v2 tags
	#[error("ID3v2: {0}")]
	Id3v2(&'static str),
	/// Arises when an invalid ID3v2 version is found
	#[error(
		"ID3v2: Found an invalid version (v{0}.{1}), expected any major revision in: (2, 3, 4)"
	)]
	BadId3v2Version(u8, u8),
	/// Arises when [`std::str::from_utf8`] fails to parse a frame ID
	#[error("ID3v2: ")]
	BadFrameID,
	/// Arises when a frame doesn't have enough data
	#[error("ID3v2: Frame isn't long enough to extract the necessary information")]
	BadFrameLength,
	/// Arises when invalid data is encountered while reading an ID3v2 synchronized text frame
	#[error("ID3v2: Encountered invalid data in SYLT frame")]
	BadSyncText,
	/// Arises when a tag is expected (Ex. found an "ID3 " chunk in a WAV file), but isn't found
	#[error("Reading: Expected a tag, found invalid data")]
	FakeTag,
	/// Arises when an atom contains invalid data
	#[error("MP4 Atom: {0}")]
	BadAtom(&'static str),
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
	/// Errors that arise while reading/writing to MP3 files
	#[error("MP3: {0}")]
	Mp3(&'static str),
	/// Errors that arise while reading/writing to MP4 files
	#[error("MP4: {0}")]
	Mp4(&'static str),
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
