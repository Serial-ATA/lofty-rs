use std::{error, fmt, io, string};

use crate::Ident;

/// Type alias for the result of tag operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Kinds of errors that may occur while performing metadata operations.
#[derive(Debug)]
pub enum ErrorKind {
    /// An error kind indicating that an atom could not be found. Contains the atom's identifier.
    AtomNotFound(Ident),
    /// An error kind indicating that an IO error has occurred. Contains the original `io::Error`.
    Io(io::Error),
    /// An error kind indicating that the filetype read from the ftyp atom was invalid. Contains
    /// the invalid filetype string.
    InvalidFiletype(String),
    /// An error kind indicating that the reader does not contain mp4 metadata.
    NoTag,
    /// An error kind indicating that an error occurred during parsing.
    Parsing,
    /// An error kind indicating that the datatype integer describing the typed data is unknown.
    /// Contains the unknown datatype.
    UnknownDataType(u32),
    /// An error kind indicating that the data can't be written to a file.
    UnWritableDataType,
    /// An error kind indicating that a string decoding error has occurred. Contains the invalid
    /// data.
    Utf8StringDecoding(string::FromUtf8Error),
    /// An error kind indicating that a string decoding error has occurred.
    Utf16StringDecoding(string::FromUtf16Error),
    /// An error kind indicating that the media type integer is unknown.
    /// Contains the unknown media type.
    UnknownMediaType(u8),
}

/// A structure able to represent any error that may occur while performing metadata operations.
pub struct Error {
    /// The kind of error.
    pub kind: ErrorKind,
    /// A human readable string describing the error.
    pub description: String,
}

impl Error {
    /// Creates a new `Error` using the error kind and description.
    pub fn new(kind: ErrorKind, description: String) -> Error {
        Error { kind, description }
    }
}

impl error::Error for Error {
    fn cause(&self) -> Option<&dyn error::Error> {
        match self.kind {
            ErrorKind::Io(ref err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error {
            kind: ErrorKind::Io(err),
            description: "".into(),
        }
    }
}

impl From<string::FromUtf8Error> for Error {
    fn from(err: string::FromUtf8Error) -> Error {
        Error {
            kind: ErrorKind::Utf8StringDecoding(err),
            description: "Data is not valid utf-8.".into(),
        }
    }
}

impl From<string::FromUtf16Error> for Error {
    fn from(err: string::FromUtf16Error) -> Error {
        Error {
            kind: ErrorKind::Utf16StringDecoding(err),
            description: "Data is not valid utf-16.".into(),
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.description.is_empty() {
            write!(f, "{:?}", self.kind)
        } else {
            write!(f, "{:?}: {}", self.kind, self.description)
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.description.is_empty() {
            write!(f, "{:?}", self.kind)
        } else {
            write!(f, "{:?}: {}", self.kind, self.description)
        }
    }
}
