pub(crate) mod ape_tag;
pub(crate) mod id3_tag;
pub(crate) mod mp4_tag;
pub(crate) mod riff_tag;
pub(crate) mod vorbis_tag;

#[cfg(feature = "ape")]
pub use ape_tag::ApeTag;
#[cfg(feature = "id3")]
pub use id3_tag::Id3v2Tag;
#[cfg(feature = "mp4")]
pub use mp4_tag::Mp4Tag;
#[cfg(feature = "riff")]
pub use riff_tag::RiffTag;
#[cfg(any(feature = "vorbis", feature = "opus", feature = "flac"))]
pub use vorbis_tag::VorbisTag;
