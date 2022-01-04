//! WAV/AIFF specific items
pub(crate) mod aiff;
pub(crate) mod chunk;
pub(crate) mod wav;

pub use aiff::AiffFile;
pub use wav::{WavFile, WavFormat, WavProperties};

#[cfg(feature = "aiff_text_chunks")]
pub use aiff::tag::{AiffTextChunks, Comment};
#[cfg(feature = "riff_info_list")]
pub use wav::tag::RiffInfoList;
