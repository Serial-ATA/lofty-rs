//! [![GitHub Workflow Status](https://img.shields.io/github/workflow/status/Serial-ATA/lofty-rs/CI?style=for-the-badge&logo=github)](https://github.com/Serial-ATA/lofty-rs/actions/workflows/ci.yml)
//! [![Downloads](https://img.shields.io/crates/d/lofty?style=for-the-badge&logo=rust)](https://crates.io/crates/lofty)
//! [![Version](https://img.shields.io/crates/v/lofty?style=for-the-badge&logo=rust)](https://crates.io/crates/lofty)
//!
//! This is a fork of [Audiotags](https://github.com/TianyiShi2001/audiotags), adding support for more file types.
//!
//! Parse, convert, and write metadata to audio formats.
//!
//! # Supported Formats
//!
//! | File Format | Extensions                                | Read | Write | Metadata Format(s)                                 |
//! |-------------|-------------------------------------------|------|-------|----------------------------------------------------|
//! | APE         | `ape`                                     |**X** |**X**  |`APEv2`, `APEv1`, `ID3v2` (Not officially), `ID3v1` |
//! | AIFF        | `aiff`, `aif`                             |**X** |**X**  |`ID3v2`, `Text Chunks`                              |
//! | FLAC        | `flac`                                    |**X** |**X**  |`Vorbis Comments`                                   |
//! | MP3         | `mp3`                                     |**X** |**X**  |`ID3v2`, `ID3v1`, `APEv2`, `APEv1`                  |
//! | MP4         | `mp4`, `m4a`, `m4b`, `m4p`, `m4v`, `isom` |**X** |**X**  |`Atoms`                                             |
//! | Opus        | `opus`                                    |**X** |**X**  |`Vorbis Comments`                                   |
//! | Ogg Vorbis  | `ogg`                                     |**X** |**X**  |`Vorbis Comments`                                   |
//! | WAV         | `wav`, `wave`                             |**X** |**X**  |`ID3v2`, `RIFF INFO`                                |
//!
//! # Examples
//!
//! ## Guessing from extension
//! ```
//! use lofty::{Tag, TagType};
//!
//! let mut tag = Tag::new().read_from_path("tests/assets/a.mp3").unwrap();
//! tag.set_title("Foo");
//!
//! assert_eq!(tag.title(), Some("Foo"));
//! ```
//!
//! ## Guessing from file signature
//! ```
//! use lofty::Tag;
//!
//! let mut tag_sig = Tag::new().read_from_path_signature("tests/assets/a.wav").unwrap();
//! tag_sig.set_artist("Foo artist");
//!
//! assert_eq!(tag_sig.artist(), Some("Foo artist"));
//! ```
//!
//! ## Specifying a TagType
//! ```
//! use lofty::{Tag, TagType};
//!
//! let mut tag = Tag::new().with_tag_type(TagType::Mp4).read_from_path("tests/assets/a.m4a").unwrap();
//! tag.set_album_title("Foo album title");
//!
//! assert_eq!(tag.album_title(), Some("Foo album title"));
//! ```
//!
//! ## Converting between TagTypes
//! ```
//! use lofty::{Tag, TagType};
//!
//! let mut tag = Tag::new().read_from_path("tests/assets/a.mp3").unwrap();
//! tag.set_title("Foo");
//!
//! // You can convert the tag type and save it to another file.
//! tag.to_dyn_tag(TagType::Mp4).write_to_path("tests/assets/a.m4a");
//! assert_eq!(tag.title(), Some("Foo"));
//! ```
//!
//! ## Converting from [`AnyTag`]
//! ```
//! use lofty::{AnyTag, OggTag, AudioTagEdit};
//!
//! let mut anytag = AnyTag::new();
//!
//! anytag.title = Some("Foo title");
//! anytag.artist = Some("Foo artist");
//!
//! let oggtag: OggTag = anytag.into();
//!
//! assert_eq!(oggtag.title(), Some("Foo title"));
//! assert_eq!(oggtag.artist(), Some("Foo artist"));
//! ```
//!
//! # Features
//!
//! ## Applies to all
//! * `all_tags` - Enables all formats
//!
//! ## Individual formats
//! These features are available if you have a specific use case, or just don't want certain formats.
//!
//! * `aiff_text_chunks`
//! * `ape`
//! * `id3v1`
//! * `id3v2`
//! * `mp4_atoms`
//! * `riff_info_list`
//! * `vorbis_comments`

#![deny(clippy::pedantic, clippy::all, missing_docs)]
#![allow(
	clippy::too_many_lines,
	clippy::cast_precision_loss,
	clippy::cast_sign_loss,
	clippy::cast_possible_wrap,
	clippy::cast_possible_truncation,
	clippy::module_name_repetitions,
	clippy::must_use_candidate,
	clippy::doc_markdown,
	clippy::let_underscore_drop,
	clippy::match_wildcard_for_single_variants,
	clippy::semicolon_if_nothing_returned,
	clippy::used_underscore_binding
)]

pub use crate::error::{LoftyError, Result};

pub use crate::probe::Probe;

pub use crate::types::{
	file::{FileType, TaggedFile},
	item::ItemKey,
	properties::FileProperties,
	tag::{ItemValue, Tag, TagItem, TagType},
};

mod types;

/// Various concrete file types, used when inference is unnecessary
pub mod files {
	pub use crate::logic::ape::ApeFile;
	pub use crate::logic::iff::{aiff::AiffFile, wav::WavFile};
	pub use crate::logic::mpeg::MpegFile;
	pub use crate::logic::ogg::{flac::FlacFile, opus::OpusFile, vorbis::VorbisFile};
}

#[cfg(any(feature = "id3v1", feature = "id3v2"))]
/// ID3v1/v2 specific items
pub mod id3 {
	pub use crate::logic::id3::v2::Id3v2Version;
}

/// Various items related to [`Picture`](crate::picture::Picture)s
pub mod picture {
	pub use crate::types::picture::{
		MimeType, Picture, PictureInformation, PictureType, TextEncoding,
	};
}

mod error;
pub(crate) mod logic;
mod probe;
