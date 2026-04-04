//! DSF (DSD Stream File) format support
//!
//! Sony's container format for DSD (Direct Stream Digital) audio.
//! The file stores 1-bit DSD samples organized in per-channel blocks,
//! with an optional ID3v2 tag appended after the audio data.

mod properties;
mod read;
pub(crate) mod write_impl;

pub use properties::DsfProperties;

use crate::id3::v2::tag::Id3v2Tag;

use lofty_attr::LoftyFile;

// Exports

/// A DSF file
#[derive(LoftyFile)]
#[lofty(read_fn = "read::read_from")]
#[lofty(internal_write_module_do_not_use_anywhere_else)]
pub struct DsfFile {
	/// An ID3v2 tag
	#[lofty(tag_type = "Id3v2")]
	pub(crate) id3v2_tag: Option<Id3v2Tag>,
	/// The file's audio properties
	pub(crate) properties: DsfProperties,
}

// DSF chunk magic signatures
pub(crate) const DSF_MAGIC: &[u8; 4] = b"DSD ";
pub(crate) const FMT_MAGIC: &[u8; 4] = b"fmt ";
pub(crate) const DATA_MAGIC: &[u8; 4] = b"data";

// Fixed chunk sizes defined by the spec
pub(crate) const HEADER_CHUNK_SIZE: u64 = 28;
pub(crate) const FMT_CHUNK_SIZE: u64 = 52;
