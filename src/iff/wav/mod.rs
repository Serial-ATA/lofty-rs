mod properties;
mod read;
pub(crate) mod write;

use crate::error::Result;
#[cfg(feature = "id3v2")]
use crate::id3::v2::tag::ID3v2Tag;
use crate::tag::{Tag, TagType};

use lofty_attr::LoftyFile;

cfg_if::cfg_if! {
	if #[cfg(feature = "riff_info_list")] {
		pub(crate) mod tag;
		use tag::RIFFInfoList;
	}
}

// Exports
pub use crate::iff::wav::properties::{WavFormat, WavProperties};

/// A WAV file
#[derive(LoftyFile)]
#[lofty(read_fn = "read::read_from")]
pub struct WavFile {
	/// A RIFF INFO LIST
	#[cfg(feature = "riff_info_list")]
	#[lofty(tag_type = "RIFFInfo")]
	pub(crate) riff_info_tag: Option<RIFFInfoList>,
	/// An ID3v2 tag
	#[cfg(feature = "id3v2")]
	#[lofty(tag_type = "ID3v2")]
	pub(crate) id3v2_tag: Option<ID3v2Tag>,
	/// The file's audio properties
	pub(crate) properties: WavProperties,
}
