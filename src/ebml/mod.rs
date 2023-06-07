//! EBML specific items
mod properties;
mod read;
mod tag;

use lofty_attr::LoftyFile;

// Exports

pub use properties::EbmlProperties;
pub use tag::EbmlTag;

/// An EBML file
#[derive(LoftyFile, Default)]
#[lofty(read_fn = "read::read_from")]
pub struct EbmlFile {
	/// An ID3v2 tag
	#[lofty(tag_type = "Id3v2")]
	pub(crate) ebml_tag: Option<EbmlTag>,
	/// The file's audio properties
	pub(crate) properties: EbmlProperties,
}
