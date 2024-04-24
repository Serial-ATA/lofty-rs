//! AAC (ADTS) specific items

// TODO: Currently we only support ADTS, might want to look into ADIF in the future.

mod header;
mod properties;
mod read;

use crate::id3::v1::tag::Id3v1Tag;
use crate::id3::v2::tag::Id3v2Tag;

use lofty_attr::LoftyFile;

// Exports

pub use properties::AACProperties;

/// An AAC (ADTS) file
#[derive(LoftyFile, Default)]
#[lofty(read_fn = "read::read_from")]
#[lofty(internal_write_module_do_not_use_anywhere_else)]
pub struct AacFile {
	#[lofty(tag_type = "Id3v2")]
	pub(crate) id3v2_tag: Option<Id3v2Tag>,
	#[lofty(tag_type = "Id3v1")]
	pub(crate) id3v1_tag: Option<Id3v1Tag>,
	pub(crate) properties: AACProperties,
}
