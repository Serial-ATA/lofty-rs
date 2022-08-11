//! MP3 specific items
mod constants;
pub(crate) mod header;
mod properties;
mod read;
pub(crate) mod write;

pub use header::{ChannelMode, Emphasis, Layer, MpegVersion};
pub use properties::MPEGProperties;

#[cfg(feature = "ape")]
use crate::ape::tag::ApeTag;
#[cfg(feature = "id3v1")]
use crate::id3::v1::tag::ID3v1Tag;
#[cfg(feature = "id3v2")]
use crate::id3::v2::tag::ID3v2Tag;

use lofty_attr::LoftyFile;

/// An MPEG file
#[derive(LoftyFile, Default)]
#[lofty(read_fn = "read::read_from")]
pub struct MPEGFile {
	/// An ID3v2 tag
	#[cfg(feature = "id3v2")]
	#[lofty(tag_type = "ID3v2")]
	pub(crate) id3v2_tag: Option<ID3v2Tag>,
	/// An ID3v1 tag
	#[cfg(feature = "id3v1")]
	#[lofty(tag_type = "ID3v1")]
	pub(crate) id3v1_tag: Option<ID3v1Tag>,
	/// An APEv1/v2 tag
	#[cfg(feature = "ape")]
	#[lofty(tag_type = "APE")]
	pub(crate) ape_tag: Option<ApeTag>,
	/// The file's audio properties
	pub(crate) properties: MPEGProperties,
}
