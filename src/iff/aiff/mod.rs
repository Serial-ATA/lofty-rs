//! AIFF specific items

mod properties;
mod read;

#[cfg(feature = "id3v2")]
use crate::id3::v2::tag::ID3v2Tag;
use crate::properties::FileProperties;

use lofty_attr::LoftyFile;

// Exports

cfg_if::cfg_if! {
	if #[cfg(feature = "aiff_text_chunks")] {
		pub(crate) mod tag;
		pub use tag::AIFFTextChunks;
		pub use tag::Comment;
	}
}

/// An AIFF file
#[derive(LoftyFile)]
#[lofty(read_fn = "read::read_from")]
#[lofty(internal_write_module_do_not_use_anywhere_else)]
pub struct AiffFile {
	/// Any text chunks included in the file
	#[cfg(feature = "aiff_text_chunks")]
	#[lofty(tag_type = "AIFFText")]
	pub(crate) text_chunks_tag: Option<AIFFTextChunks>,
	/// An ID3v2 tag
	#[cfg(feature = "id3v2")]
	#[lofty(tag_type = "ID3v2")]
	pub(crate) id3v2_tag: Option<ID3v2Tag>,
	/// The file's audio properties
	pub(crate) properties: FileProperties,
}
