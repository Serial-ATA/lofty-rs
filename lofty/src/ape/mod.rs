//! APE specific items
//!
//! ## File notes
//!
//! It is possible for an `APE` file to contain an `ID3v2` tag. For the sake of data preservation,
//! this tag will be read, but **cannot** be written. The only tags allowed by spec are `APEv1/2` and
//! `ID3v1`.
pub(crate) mod constants;
pub(crate) mod header;
mod properties;
mod read;
pub(crate) mod tag;

use crate::id3::v1::tag::Id3v1Tag;
use crate::id3::v2::tag::Id3v2Tag;

use lofty_attr::LoftyFile;

// Exports

pub use crate::picture::APE_PICTURE_TYPES;
pub use properties::ApeProperties;
pub use tag::ApeTag;
pub use tag::item::ApeItem;

/// An APE file
#[derive(LoftyFile)]
#[lofty(read_fn = "read::read_from")]
#[lofty(internal_write_module_do_not_use_anywhere_else)]
pub struct ApeFile {
	/// An ID3v1 tag
	#[lofty(tag_type = "Id3v1")]
	pub(crate) id3v1_tag: Option<Id3v1Tag>,
	/// An ID3v2 tag (Not officially supported)
	#[lofty(tag_type = "Id3v2")]
	pub(crate) id3v2_tag: Option<Id3v2Tag>,
	/// An APEv1/v2 tag
	#[lofty(tag_type = "Ape")]
	pub(crate) ape_tag: Option<ApeTag>,
	/// The file's audio properties
	pub(crate) properties: ApeProperties,
}
