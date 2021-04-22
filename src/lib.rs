//! [![Crate](https://img.shields.io/crates/v/lofty.svg)](https://crates.io/crates/lofty)
//! [![Crate](https://img.shields.io/crates/d/lofty.svg)](https://crates.io/crates/lofty)
//! [![Crate](https://img.shields.io/crates/l/lofty.svg)](https://crates.io/crates/lofty)
//! [![Documentation](https://docs.rs/lofty/badge.svg)](https://docs.rs/lofty/)
//!
//! This is a fork of [Audiotags](https://github.com/TianyiShi2001/audiotags), adding support for more file types and (optionally) duration.
//!
//! Parse, convert, and write metadata to audio files of different file types.
//!
//! This crate aims to provide a unified trait for parsers and writers of different audio file formats.
//! Without this crate, you would otherwise need to learn the different APIs in **id3**, **mp4ameta**, etc.
//! in order to parse metadata in different file formats.
//!
//! # Supported Formats
//!
//! | File Format | Extensions                                | Read | Write | Metadata Format(s)   |
//! |-------------|-------------------------------------------|------|-------|----------------------|
//! | Ape         | `ape`                                     |**X** |**X**  | `APEv2`              |
//! | AIFF        | `aiff`, `aif`                             |**X** |**X**  | `ID3v2`              |
//! | FLAC        | `flac`                                    |**X** |**X**  | `Vorbis Comments`    |
//! | MP3         | `mp3`                                     |**X** |**X**  | `ID3v2`              |
//! | MP4         | `mp4`, `m4a`, `m4b`, `m4p`, `m4v`, `isom` |**X** |**X**  | `Vorbis Comments`    |
//! | Opus        | `opus`                                    |**X** |       | `Vorbis Comments`    |
//! | Ogg         | `ogg`, `oga`                              |**X** |**X**  | `Vorbis Comments`    |
//! | WAV         | `wav`, `wave`                             |**X** |**X**  | `RIFF INFO`, `ID3v2` |
//!
//! # Examples
//!
//! ```
//! use lofty::{Tag, TagType};
//!
//! // Guess the format from the extension, in this case `mp3`
//! let mut tag = Tag::new().read_from_path("tests/assets/a.mp3").unwrap();
//! tag.set_title("Foo");
//!
//! // You can also guess the format from the file signature
//! let mut tag_sig = Tag::new().read_from_path_signature("tests/assets/a.wav").unwrap();
//! tag_sig.set_artist("Foo artist");
//!
//! // You can convert the tag type and save the metadata to another file.
//! tag.to_dyn_tag(TagType::Mp4).write_to_path("tests/assets/a.m4a");
//!
//! // You can specify the tag type, but when you want to do this
//! // also consider directly using the concrete type
//! let tag = Tag::new().with_tag_type(TagType::Mp4).read_from_path("tests/assets/a.m4a").unwrap();
//! assert_eq!(tag.title(), Some("Foo"));
//! ```
//!
//! # Features
//!
//! By default, `full` (`all_tags` and `duration`) are enabled.
//!
//! `all_tags` provides all tag types (ID3, RIFF, Vorbis, etc).
//!
//! `duration` provides the `duration` field in each tag (ex. Tag.duration).
//!
//! Either one can be disabled if it doesn't fit your use case.
//!
//! In addition to this, each format can be individually enabled.
//! All features are: `ape, mp3, mp4, vorbis, wav`.
//!
//! ## Performance
//!
//! Using lofty incurs a little overhead due to vtables if you want to guess the metadata format (from file extension).
//! Apart from this, the performance is almost the same as directly calling the function provided from those ‘specialized’ crates.
//!
//! No copies will be made if you only need to read and write metadata of one format. If you want to convert between tags, copying is
//! unavoidable, no matter if you use lofty or use getters and setters provided by specialized libraries. Lofty is not making additional
//! unnecessary copies.

#![warn(clippy::pedantic)]
#![allow(
	clippy::too_many_lines,
	clippy::cast_precision_loss,
	clippy::cast_sign_loss,
	clippy::cast_possible_wrap,
	clippy::clippy::cast_possible_truncation,
	clippy::module_name_repetitions,
	clippy::must_use_candidate,
	clippy::doc_markdown,
	clippy::let_underscore_drop,
	clippy::match_wildcard_for_single_variants
)]

#[doc(hidden)]
mod macros;

mod types;
pub use crate::types::{
	album::Album,
	anytag::AnyTag,
	picture::{MimeType, Picture},
};

mod tag;
pub use crate::tag::{ID3Format, Tag, TagType, VorbisFormat};

mod error;
pub use crate::error::{Error, Result};

mod components;
pub use crate::components::tags::*;

mod traits;
pub use crate::traits::{AudioTag, AudioTagEdit, AudioTagWrite, ToAny, ToAnyTag};
