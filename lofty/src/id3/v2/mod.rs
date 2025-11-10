//! ID3v2 items and utilities
//!
//! ## Important notes
//!
//! See:
//!
//! * [`Id3v2Tag`]
//! * [`Frame`]

mod frame;
pub(crate) mod header;
mod items;
pub(crate) mod read;
mod restrictions;
pub(crate) mod tag;
pub mod util;
pub(crate) mod write;

// Exports

pub use header::{Id3v2TagFlags, Id3v2Version};
pub use util::upgrade::{upgrade_v2, upgrade_v3};

pub use tag::Id3v2Tag;

pub use items::*;

pub use frame::header::{FrameHeader, FrameId};
pub use frame::{Frame, FrameFlags};

pub use restrictions::{
	ImageSizeRestrictions, TagRestrictions, TagSizeRestrictions, TextSizeRestrictions,
};

/// ID3v2 [`TextEncoding`] extensions
pub(crate) trait Id3TextEncodingExt {
	/// ID3v2.4 introduced two new text encodings.
	///
	/// When writing ID3v2.3, we just substitute with UTF-16.
	fn to_id3v23(self) -> Self;
}

impl Id3TextEncodingExt for aud_io::text::TextEncoding {
	fn to_id3v23(self) -> Self {
		match self {
			Self::UTF8 | Self::UTF16BE => {
				log::warn!(
					"Text encoding {:?} is not supported in ID3v2.3, substituting with UTF-16",
					self
				);
				Self::UTF16
			},
			_ => self,
		}
	}
}
