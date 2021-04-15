pub(crate) mod ape_tag;
pub(crate) mod id3_tag;
pub(crate) mod mp4_tag;
pub(crate) mod vorbis_tag;
pub(crate) mod wav_tag;

#[cfg(feature = "ape")]
pub use ape_tag::ApeTag;
#[cfg(feature = "mp3")]
pub use id3_tag::Id3v2Tag;
#[cfg(feature = "mp4")]
pub use mp4_tag::Mp4Tag;
#[cfg(feature = "vorbis")]
pub use vorbis_tag::VorbisTag;
