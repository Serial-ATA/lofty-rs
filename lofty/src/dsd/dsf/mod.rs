//! DSF (DSD Stream File) format support
//!
//! Sony's format for DSD audio with ID3v2 tag support

mod properties;
mod read;
pub(crate) mod write_impl;

pub use properties::DsfProperties;

use crate::id3::v2::Id3v2Tag;

use lofty_attr::LoftyFile;

/// DSF file representation
#[derive(LoftyFile, Debug)]
#[lofty(read_fn = "read::read_from")]
#[lofty(internal_write_module_do_not_use_anywhere_else)]
pub struct DsfFile {
	/// ID3v2 tag
	#[lofty(tag_type = "Id3v2")]
	pub(crate) id3v2_tag: Option<Id3v2Tag>,
	/// The file's audio properties
	pub(crate) properties: DsfProperties,
}

// DSF file structure constants
pub(crate) const DSF_MAGIC: &[u8; 4] = b"DSD ";
pub(crate) const FMT_MAGIC: &[u8; 4] = b"fmt ";
pub(crate) const DATA_MAGIC: &[u8; 4] = b"data";

pub(crate) const HEADER_SIZE: u64 = 28;
pub(crate) const FMT_CHUNK_SIZE: u64 = 52;
