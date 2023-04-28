//! MP3 specific items
mod constants;
pub(crate) mod header;
mod properties;
mod read;

pub use header::{ChannelMode, Emphasis, Layer, MpegVersion};
pub use properties::MpegProperties;

use crate::ape::tag::ApeTag;
use crate::id3::v1::tag::Id3v1Tag;
use crate::id3::v2::tag::Id3v2Tag;

use lofty_attr::LoftyFile;

/// An MPEG file
#[derive(LoftyFile, Default)]
#[lofty(read_fn = "read::read_from")]
#[lofty(internal_write_module_do_not_use_anywhere_else)]
pub struct MpegFile {
	/// An ID3v2 tag
	#[lofty(tag_type = "Id3v2")]
	pub(crate) id3v2_tag: Option<Id3v2Tag>,
	/// An ID3v1 tag
	#[lofty(tag_type = "Id3v1")]
	pub(crate) id3v1_tag: Option<Id3v1Tag>,
	/// An APEv1/v2 tag
	#[lofty(tag_type = "Ape")]
	pub(crate) ape_tag: Option<ApeTag>,
	/// The file's audio properties
	pub(crate) properties: MpegProperties,
}
