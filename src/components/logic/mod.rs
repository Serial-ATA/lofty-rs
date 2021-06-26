#[cfg(any(feature = "format-opus", feature = "format-vorbis", feature = "format-riff"))]
pub(crate) mod constants;

#[cfg(any(feature = "format-opus", feature = "format-vorbis"))]
pub(crate) mod ogg_generic;

#[cfg(feature = "format-flac")]
pub(crate) mod flac;

#[cfg(feature = "format-riff")]
pub(crate) mod riff;
