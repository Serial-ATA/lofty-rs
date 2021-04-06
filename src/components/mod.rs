pub(crate) mod flac_tag;
pub(crate) mod id3_tag;
pub(crate) mod mp4_tag;
pub(crate) mod opus_tag;
pub(crate) mod ogg_tag;

pub use flac_tag::FlacTag;
pub use id3_tag::Id3v2Tag;
pub use mp4_tag::Mp4Tag;
pub use opus_tag::OpusTag;
pub use ogg_tag::OggTag;
