/// Error types that could occur in this library.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("")]
    DowncastError,

    /// Fail to guess the metadata format based on the file extension.
    #[error("Fail to guess the metadata format based on the file extension.")]
    UnknownFileExtension(String),

    /// Represents a failure to read from input.
    #[error("Read error")]
    ReadError { source: std::io::Error },

    /// Represents all other cases of `std::io::Error`.
    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    #[error("Unsupported mime type: {0}")]
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

pub type Result<T> = std::result::Result<T, Error>;
