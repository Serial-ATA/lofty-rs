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
//! | File Format | Extensions                                | Read | Write | Metadata Format(s)  |
//! |-------------|-------------------------------------------|------|-------|---------------------|
//! | Ape         | `ape`                                     |**X** |**X**  |`APEv2`              |
//! | AIFF        | `aiff`, `aif`                             |**X** |**X**  |`ID3v2`              |
//! | FLAC        | `flac`                                    |**X** |**X**  |`Vorbis Comments`    |
//! | MP3         | `mp3`                                     |**X** |**X**  |`ID3v2`              |
//! | MP4         | `mp4`, `m4a`, `m4b`, `m4p`, `m4v`, `isom` |**X** |**X**  |`Vorbis Comments`    |
//! | Opus        | `opus`                                    |**X** |**X**  |`Vorbis Comments`    |
//! | Ogg         | `ogg`, `oga`                              |**X** |**X**  |`Vorbis Comments`    |
//! | WAV         | `wav`, `wave`                             |**X** |**X**  |`RIFF INFO`, `ID3v2` |
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
//! assert_eq!(tag_sig.artist_str(), Some("Foo artist"));
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
//! // You can convert the tag type and save the metadata to another file.
//! tag.to_dyn_tag(TagType::Mp4).write_to_path("tests/assets/a.m4a");
//! assert_eq!(tag.title(), Some("Foo"));
//! ```
//!
//! # Features
//!
//! ## Applies to all
//! * `all_tags` - Enables all formats
//!
//! ## Individual formats
//! These features are available if you have a specific usecase, or just don't want certain formats.
//!
//! All format features a prefixed with `format-`
//! * `format-ape`
//! * `format-flac`
//! * `format-id3`
//! * `format-mp4`
//! * `format-opus`
//! * `format-vorbis`
//! * `format-riff`
//!
//! ## Umbrella features
//! These cover all formats under a container format.
//!
//! * `format-ogg` (`format-opus`, `format-vorbis`, `format-flac`)

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
	clippy::match_wildcard_for_single_variants
)]

mod types;
pub use crate::types::{
	album::Album,
	anytag::AnyTag,
	picture::{MimeType, Picture, PictureType},
};

mod tag;
pub use crate::tag::{Id3Format, OggFormat, Tag, TagType};

mod error;
pub use crate::error::{LoftyError, Result};

mod components;
pub use crate::components::tags::*;

mod traits;
pub use crate::traits::{AudioTag, AudioTagEdit, AudioTagWrite, ToAny, ToAnyTag};
