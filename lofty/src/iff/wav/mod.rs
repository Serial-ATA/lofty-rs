//! WAV specific items

mod properties;
pub(crate) mod read;
pub(crate) mod tag;

use crate::id3::v2::tag::Id3v2Tag;

use lofty_attr::LoftyFile;

// Exports
pub use crate::iff::wav::properties::{WavFormat, WavProperties};
pub use tag::RiffInfoList;

/// A WAV file
#[derive(LoftyFile)]
#[lofty(read_fn = "read::read_from")]
#[lofty(internal_write_module_do_not_use_anywhere_else)]
pub struct WavFile {
	/// A RIFF INFO LIST
	#[lofty(tag_type = "RiffInfo")]
	pub(crate) riff_info_tag: Option<RiffInfoList>,
	/// An ID3v2 tag
	#[lofty(tag_type = "Id3v2")]
	pub(crate) id3v2_tag: Option<Id3v2Tag>,
	/// The file's audio properties
	pub(crate) properties: WavProperties,
}
