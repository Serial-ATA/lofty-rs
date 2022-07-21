mod properties;
mod read;
pub(crate) mod write;

#[cfg(feature = "id3v2")]
use crate::id3::v2::tag::ID3v2Tag;
use crate::properties::FileProperties;
use crate::tag::{Tag, TagType};

use lofty_attr::LoftyFile;

cfg_if::cfg_if! {
	if #[cfg(feature = "aiff_text_chunks")] {
		pub(crate) mod tag;
		use tag::AIFFTextChunks;
	}
}

/// An AIFF file
#[derive(LoftyFile)]
#[lofty(read_fn = "read::read_from")]
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
