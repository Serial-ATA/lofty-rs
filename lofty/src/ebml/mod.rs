//! EBML specific items
mod element_reader;
mod properties;
mod read;
pub(crate) mod tag;
mod vint;

use lofty_attr::LoftyFile;

// Exports

pub use properties::*;
pub use tag::*;
pub use vint::*;

/// An EBML file
#[derive(LoftyFile, Default)]
#[lofty(read_fn = "read::read_from")]
#[lofty(internal_write_module_do_not_use_anywhere_else)]
pub struct EbmlFile {
	/// An EBML tag
	#[lofty(tag_type = "Matroska")]
	pub(crate) ebml_tag: Option<MatroskaTag>,
	/// The file's audio properties
	pub(crate) properties: EbmlProperties,
}