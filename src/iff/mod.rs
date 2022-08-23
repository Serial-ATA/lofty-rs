//! WAV/AIFF specific items
pub(crate) mod aiff;
pub(crate) mod chunk;
pub(crate) mod wav;

// TODO: Expose `iff::{aiff, wav}` instead of combining both here

// Exports

pub use aiff::AiffFile;
pub use wav::{WavFile, WavFormat, WavProperties};

cfg_if::cfg_if! {
	if #[cfg(feature = "aiff_text_chunks")] {
		pub use aiff::tag::AIFFTextChunks;
		pub use aiff::tag::Comment;
	}
}

cfg_if::cfg_if! {
	if #[cfg(feature = "riff_info_list")] {
		pub use wav::tag::RIFFInfoList;
	}
}
