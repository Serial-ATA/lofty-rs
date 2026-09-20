//! DSD Stream File (DSF) specific items

mod error;
mod properties;
pub(crate) mod read;

use crate::id3::v2::tag::Id3v2Tag;

use lofty_attr::LoftyFile;

// Exports

pub use properties::{DsfProperties, FormatId};

/// A DSD Stream File (DSF) file
#[derive(LoftyFile, Default)]
#[lofty(read_fn = "read::read_from")]
#[lofty(internal_write_module_do_not_use_anywhere_else)]
pub struct DsfFile {
	#[lofty(tag_type = "Id3v2")]
	pub(crate) id3v2_tag: Option<Id3v2Tag>,
	pub(crate) properties: DsfProperties,
}
