//! Generic file handling utilities

mod audio_file;
mod file_type;
mod tagged_file;

pub use audio_file::AudioFile;
pub use file_type::{EXTENSIONS, FileType};
pub use tagged_file::{BoundTaggedFile, TaggedFile, TaggedFileExt};

pub(crate) use file_type::FileTypeGuessResult;
