//! EBML specific items
mod properties;
mod read;
mod tag;
mod vint;

use lofty_attr::LoftyFile;

// Exports

pub use properties::EbmlProperties;
pub use tag::EbmlTag;
pub use vint::VInt;

/// An EBML file
#[derive(LoftyFile, Default)]
#[lofty(read_fn = "read::read_from")]
// TODO: #[lofty(internal_write_module_do_not_use_anywhere_else)]
pub struct EbmlFile {
	/// An EBML tag
	#[lofty(tag_type = "Ebml")]
	pub(crate) ebml_tag: Option<EbmlTag>,
	/// The file's audio properties
	pub(crate) properties: EbmlProperties,
}
