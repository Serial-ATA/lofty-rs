#[cfg(any(
	feature = "format-opus",
	feature = "format-vorbis",
	feature = "format-flac"
))]
pub(crate) mod ogg;

#[cfg(any(
	feature = "format-aiff",
	feature = "format-riff",
	feature = "format-id3"
))]
pub(crate) mod iff;

#[cfg(feature = "format-id3")] // TODO: new feature?
pub(crate) mod mpeg;
