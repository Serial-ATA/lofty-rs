//! WAV/AIFF specific items
pub(crate) mod aiff;
pub(crate) mod chunk;
pub(crate) mod wav;

use crate::macros::feature_locked;

// Exports

pub use aiff::AiffFile;
pub use wav::{WavFile, WavFormat, WavProperties};

feature_locked! {
	#![cfg(feature = "aiff_text_chunks")]

	pub use aiff::tag::AiffTextChunks;
	pub use aiff::tag::Comment;
}

feature_locked! {
	#![cfg(feature = "riff_info_list")]

	pub use wav::tag::RiffInfoList;
}
