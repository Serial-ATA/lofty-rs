//! Items for FLAC
//!
//! ## File notes
//!
//! * See [`FlacFile`]

pub(crate) mod block;
pub(crate) mod properties;
mod read;
#[cfg(feature = "vorbis_comments")]
pub(crate) mod write;

#[cfg(feature = "id3v2")]
use crate::id3::v2::tag::ID3v2Tag;
#[cfg(feature = "vorbis_comments")]
use crate::ogg::VorbisComments;
use crate::properties::FileProperties;

use lofty_attr::LoftyFile;

/// A FLAC file
///
/// ## Notes
///
/// * The ID3v2 tag is **read only**, and it's use is discouraged by spec
/// * Picture blocks will be stored in the `VorbisComments` tag, meaning a file could have no vorbis
///   comments block, but `FlacFile::vorbis_comments` will exist.
///   * When writing, the pictures will be stored in their own picture blocks
///   * This behavior will likely change in the future
#[derive(LoftyFile)]
#[lofty(read_fn = "read::read_from")]
pub struct FlacFile {
	/// An ID3v2 tag
	#[cfg(feature = "id3v2")]
	#[lofty(tag_type = "ID3v2")]
	pub(crate) id3v2_tag: Option<ID3v2Tag>,
	/// The vorbis comments contained in the file
	///
	/// NOTE: This field being `Some` does not mean the file has vorbis comments, as Picture blocks exist.
	#[cfg(feature = "vorbis_comments")]
	#[lofty(tag_type = "VorbisComments")]
	pub(crate) vorbis_comments_tag: Option<VorbisComments>,
	/// The file's audio properties
	pub(crate) properties: FileProperties,
}
