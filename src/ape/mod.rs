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
pub(crate) mod write;

#[cfg(feature = "id3v1")]
use crate::id3::v1::tag::ID3v1Tag;
#[cfg(feature = "id3v2")]
use crate::id3::v2::tag::ID3v2Tag;

use lofty_attr::LoftyFile;

// Exports

cfg_if::cfg_if! {
	if #[cfg(feature = "ape")] {
		pub(crate) mod tag;
		pub use tag::ApeTag;
		pub use tag::item::ApeItem;

		pub use crate::picture::APE_PICTURE_TYPES;
	}
}

pub use properties::ApeProperties;

/// An APE file
#[derive(LoftyFile)]
#[lofty(read_fn = "read::read_from")]
pub struct ApeFile {
	/// An ID3v1 tag
	#[cfg(feature = "id3v1")]
	#[lofty(tag_type = "ID3v1")]
	pub(crate) id3v1_tag: Option<ID3v1Tag>,
	/// An ID3v2 tag (Not officially supported)
	#[cfg(feature = "id3v2")]
	#[lofty(tag_type = "ID3v2")]
	pub(crate) id3v2_tag: Option<ID3v2Tag>,
	/// An APEv1/v2 tag
	#[cfg(feature = "ape")]
	#[lofty(tag_type = "APE")]
	pub(crate) ape_tag: Option<ApeTag>,
	/// The file's audio properties
	pub(crate) properties: ApeProperties,
}
