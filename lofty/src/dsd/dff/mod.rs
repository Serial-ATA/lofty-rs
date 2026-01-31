//! DFF (DSDIFF - DSD Interchange File Format) support
//!
//! Philips' IFF-based format for DSD audio with ID3v2 and text chunk support

mod properties;
mod read;
pub(crate) mod tag;
pub(crate) mod write;

pub use properties::{DffProperties, LoudspeakerConfig};
pub use read::read_from;
pub use tag::{DffEditedMasterInfo, DffTextChunks};

use crate::id3::v2::Id3v2Tag;

use lofty_attr::LoftyFile;

/// DFF file representation
#[derive(LoftyFile)]
#[lofty(read_fn = "read::read_from")]
#[lofty(write_fn = "write::write_dff_file")]
pub struct DffFile {
	/// DFF text chunks (DIIN)
	#[lofty(tag_type = "DffText")]
	pub(crate) dff_text_tag: Option<DffTextChunks>,
	/// ID3v2 tag
	#[lofty(tag_type = "Id3v2")]
	pub(crate) id3v2_tag: Option<Id3v2Tag>,
	/// The file's audio properties
	pub(crate) properties: DffProperties,
}
