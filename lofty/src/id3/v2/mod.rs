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
