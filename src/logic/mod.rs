pub(crate) mod ape;
pub(crate) mod iff;
pub(crate) mod mpeg;
pub(crate) mod ogg;

#[cfg(any(feature = "id3v1", feature = "id3v2"))]
pub(crate) mod id3;
