//! [![GitHub Workflow Status](https://img.shields.io/github/workflow/status/Serial-ATA/lofty-rs/CI?style=for-the-badge&logo=github)](https://github.com/Serial-ATA/lofty-rs/actions/workflows/ci.yml)
//! [![Downloads](https://img.shields.io/crates/d/lofty?style=for-the-badge&logo=rust)](https://crates.io/crates/lofty)
//! [![Version](https://img.shields.io/crates/v/lofty?style=for-the-badge&logo=rust)](https://crates.io/crates/lofty)
//!
//! Parse, convert, and write metadata to audio formats.
//!
//! # Supported Formats
//!
//! | File Format | Extensions                                      | Read | Write | Metadata Format(s)                                 |
//! |-------------|-------------------------------------------------|------|-------|----------------------------------------------------|
//! | APE         | `ape`                                           |**X** |**X**  |`APEv2`, `APEv1`, `ID3v2` (Not officially), `ID3v1` |
//! | AIFF        | `aiff`, `aif`                                   |**X** |**X**  |`ID3v2`, `Text Chunks`                              |
//! | FLAC        | `flac`                                          |**X** |**X**  |`Vorbis Comments`                                   |
//! | MP3         | `mp3`                                           |**X** |**X**  |`ID3v2`, `ID3v1`, `APEv2`, `APEv1`                  |
//! | MP4         | `mp4`, `m4a`, `m4b`, `m4p`, `m4r`, `m4v`, `3gp` |**X** |**X**  |`Atoms`                                             |
//! | Opus        | `opus`                                          |**X** |**X**  |`Vorbis Comments`                                   |
//! | Ogg Vorbis  | `ogg`                                           |**X** |**X**  |`Vorbis Comments`                                   |
//! | WAV         | `wav`, `wave`                                   |**X** |**X**  |`ID3v2`, `RIFF INFO`                                |
//!
//! # Examples
//!
//! ## Determining a file's format
//!
//! These don't read the file's properties or tags. Instead, they determine the [`FileType`], which is useful for matching against [`concrete file types`](crate::files).
//!
//! ### Guessing from extension
//! ```
//! use lofty::{Probe, FileType};
//!
//! let file_type = Probe::new().file_type_from_extension("tests/assets/a.mp3").unwrap();
//!
//! assert_eq!(file_type, FileType::MP3)
//! ```
//!
//! ### Guessing from file content
//! ```
//! use lofty::{Probe, FileType};
//!
//! // Probe::file_type also exists for generic readers
//! let file_type = Probe::new().file_type_from_path("tests/assets/a.mp3").unwrap();
//!
//! assert_eq!(file_type, FileType::MP3)
//! ```
//!
//! ## Using concrete file types
//! ```
//! use lofty::files::MpegFile;
//! use lofty::TagType;
//! use std::fs::File;
//!
//! let mut file_content = File::open("tests/assets/a.mp3");
//!
//! let mpeg_file = MpegFile::read_from(&mut file_content).unwrap();
//!
//! assert_eq!(mpeg_file.properties().channels(), Some(2));
//! assert!(mpeg_file.contains_tag_type(&TagType::Ape));
//! ```
//!
//! ## Non-specific tagged files
//!
//! These are useful if the file format doesn't matter
//!
//! ### Reading
//! ```
//! use lofty::{Probe, FileType};
//!
//! // Probe::read_from also exists for generic readers
//! let tagged_file = Probe::new().read_from_path("tests/assets/a.mp3").unwrap();
//!
//! assert_eq!(tagged_file.file_type(), FileType::MP3);
//! assert_eq!(tagged_file.properties().channels(), Some(2));
//! ```
//!
//! ### Accessing tags
//! ```
//! use lofty::Probe;
//!
//! let tagged_file = Probe::new().read_from_path("tests/assets/a.mp3").unwrap();
//!
//! // Get the primary tag (ID3v2 in this case)
//! let id3v2 = tagged_file.primary_tag().unwrap();
//!
//! // If the primary tag doesn't exist, or the tag types
//! // don't matter, the first tag can be retrieved
//! let unknown_first_tag = tagged_file.first_tag().unwrap();
//! ```
//!
//! # Features
//!
//! NOTE: All of these are enabled by default
//!
//! ## QOL
//! * `quick_tag_accessors` - Adds easier getters/setters for string values (Ex. [`Tag::artist`]), adds an extra dependency
//!
//! ## Individual metadata formats
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
	pub use crate::logic::id3::v2::util::sync_text::{
		SyncTextContentType, SynchronizedText, TimestampFormat,
	};
	pub use crate::logic::id3::v2::util::upgrade::{upgrade_v2, upgrade_v3};
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
