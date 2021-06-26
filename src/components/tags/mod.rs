#[cfg(feature = "format-ape")]
pub(crate) mod ape_tag;
#[cfg(feature = "format-ape")]
pub use ape_tag::ApeTag;

#[cfg(feature = "format-id3")]
pub(crate) mod id3_tag;
#[cfg(feature = "format-id3")]
pub use id3_tag::Id3v2Tag;

#[cfg(feature = "format-mp4")]
pub(crate) mod mp4_tag;
#[cfg(feature = "format-mp4")]
pub use mp4_tag::Mp4Tag;

#[cfg(feature = "format-riff")]
pub(crate) mod riff_tag;
#[cfg(feature = "format-riff")]
pub use riff_tag::RiffTag;

#[cfg(any(
	feature = "format-vorbis",
	feature = "format-opus",
	feature = "format-flac"
))]
pub(crate) mod vorbis_tag;
#[cfg(any(
	feature = "format-vorbis",
	feature = "format-opus",
	feature = "format-flac"
))]
pub use vorbis_tag::VorbisTag;
