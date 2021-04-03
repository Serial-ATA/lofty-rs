/// Errors that could occur in this library.
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
	Id3TagError(#[from] id3::Error),
	#[error(transparent)]
	VorbisError(#[from] lewton::VorbisError),
	#[error(transparent)]
	FlacTagError(#[from] metaflac::Error),
	#[error(transparent)]
	Mp4TagError(#[from] mp4ameta::Error),

	#[error("")]
	NotAPicture,

	/// Represents all cases of `std::io::Error`.
	#[error(transparent)]
	IOError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
