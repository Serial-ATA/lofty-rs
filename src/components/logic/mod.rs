#[cfg(feature = "format-riff")]
pub(crate) mod riff;

#[cfg(any(
	feature = "format-opus",
	feature = "format-vorbis",
	feature = "format-flac"
))]
pub(crate) mod ogg;

#[cfg(feature = "format-aiff")]
pub(crate) mod aiff;
