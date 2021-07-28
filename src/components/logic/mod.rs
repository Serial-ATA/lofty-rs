#[cfg(any(
	feature = "format-opus",
	feature = "format-vorbis",
	feature = "format-flac"
))]
pub(crate) mod ogg;

#[cfg(any(feature = "format-aiff", feature = "format-id3"))]
pub(crate) mod iff;
