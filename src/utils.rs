//use strum::Display;
use thiserror::Error;

/// Error types that could occur in this library.
#[derive(Error, Debug)]
pub enum AudioTagsError {
    /// Fail to guess the metadata format based on the file extension.
    #[error("Fail to guess the metadata format based on the file extension.")]
    UnknownFileExtension(String),

    /// Represents a failure to read from input.
    #[error("Read error")]
    ReadError { source: std::io::Error },

    /// Represents all other cases of `std::io::Error`.
    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error("")]
    UnsupportedFormat(String),
    #[error("")]
    UnsupportedMimeType(String),
    #[error("")]
    NotAPicture,

    #[error(transparent)]
    FlacTagError(#[from] metaflac::Error),

    #[error(transparent)]
    Mp4TagError(#[from] mp4ameta::Error),

    #[error(transparent)]
    Id3TagError(#[from] id3::Error),
}

pub type AudioTagsResult<T> = Result<T, AudioTagsError>;
