//! AIFF specific items

mod properties;
mod read;
pub(crate) mod tag;

use crate::id3::v2::tag::Id3v2Tag;

use lofty_attr::LoftyFile;

// Exports

pub use properties::{AiffCompressionType, AiffProperties};
pub use tag::{AiffTextChunks, Comment};

/// An AIFF file
#[derive(LoftyFile)]
#[lofty(read_fn = "read::read_from")]
#[lofty(internal_write_module_do_not_use_anywhere_else)]
pub struct AiffFile {
	/// Any text chunks included in the file
	#[lofty(tag_type = "AiffText")]
	pub(crate) text_chunks_tag: Option<AiffTextChunks>,
	/// An ID3v2 tag
	#[lofty(tag_type = "Id3v2")]
	pub(crate) id3v2_tag: Option<Id3v2Tag>,
	/// The file's audio properties
	pub(crate) properties: AiffProperties,
}
