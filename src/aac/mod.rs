//! AAC (ADTS) specific items

mod properties;
mod read;

use crate::ape::tag::ApeTag;
use crate::id3::v1::tag::ID3v1Tag;
use crate::id3::v2::tag::ID3v2Tag;
use properties::AACProperties;

use lofty_attr::LoftyFile;

/// An AAC (ADTS) file
#[derive(LoftyFile, Default)]
#[lofty(read_fn = "read::read_from")]
pub struct AACFile {
	#[cfg(feature = "id3v2")]
	#[lofty(tag_type = "ID3v2")]
	pub(crate) id3v2_tag: Option<ID3v2Tag>,
	#[cfg(feature = "id3v1")]
	#[lofty(tag_type = "ID3v1")]
	pub(crate) id3v1_tag: Option<ID3v1Tag>,
	#[cfg(feature = "ape")]
	#[lofty(tag_type = "APE")]
	pub(crate) ape_tag: Option<ApeTag>,
	pub(crate) properties: AACProperties,
}
