//! [![GitHub Workflow Status](https://img.shields.io/github/workflow/status/Serial-ATA/lofty-rs/CI?style=for-the-badge&logo=github)](https://github.com/Serial-ATA/lofty-rs/actions/workflows/ci.yml)
//! [![Downloads](https://img.shields.io/crates/d/lofty?style=for-the-badge&logo=rust)](https://crates.io/crates/lofty)
//! [![Version](https://img.shields.io/crates/v/lofty?style=for-the-badge&logo=rust)](https://crates.io/crates/lofty)
//!
//! Parse, convert, and write metadata to audio formats.
//!
//! # Supported Formats
//!
//! [See here](https://github.com/Serial-ATA/lofty-rs#supported-formats)
//!
//! # Examples
//!
//! ## Reading a generic file
//!
//! It isn't always convenient to [use concrete file types](#using-concrete-file-types), which is where [`TaggedFile`]
//! comes in.
//!
//! ### Using a path
//!
//! ```rust,no_run
//! # use lofty::LoftyError;
//! # fn main() -> Result<(), LoftyError> {
//! use lofty::{read_from_path, Probe};
//!
//! // This will guess the format from the extension
//! // ("mp3" in this case), but we can guess from the content if we want to.
//! let path = "test.mp3";
//! let tagged_file = read_from_path(path)?;
//!
//! // Let's guess the format from the content just in case.
//! // This is not necessary in this case!
//! let tagged_file2 = Probe::open(path)?.guess_file_type()?.read()?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Using an existing reader
//!
//! ```rust,no_run
//! # use lofty::LoftyError;
//! # fn main() -> Result<(), LoftyError> {
//! use lofty::{read_from, ParseOptions};
//! use std::fs::File;
//!
//! // Let's read from an open file
//! let path = "test.mp3";
//! let mut file = File::open(path)?;
//!
//! // Here, we have to guess the file type prior to reading
//! let tagged_file = read_from(&mut file)?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Accessing tags
//!
//! ```rust,no_run
//! # use lofty::LoftyError;
//! # fn main() -> Result<(), LoftyError> {
//! use lofty::{read_from_path, ParseOptions};
//!
//! let path = "test.mp3";
//! let tagged_file = read_from_path(path)?;
//!
//! // Get the primary tag (ID3v2 in this case)
//! let id3v2 = tagged_file.primary_tag();
//!
//! // If the primary tag doesn't exist, or the tag types
//! // don't matter, the first tag can be retrieved
//! let unknown_first_tag = tagged_file.first_tag();
//! # Ok(())
//! # }
//! ```
//!
//! ## Using concrete file types
//!
//! ```rust
//! # use lofty::LoftyError;
//! # fn main() -> Result<(), LoftyError> {
//! use lofty::mpeg::MPEGFile;
//! use lofty::{AudioFile, ParseOptions, TagType};
//! use std::fs::File;
//!
//! # let path = "tests/files/assets/minimal/full_test.mp3";
//! let mut file_content = File::open(path)?;
//!
//! // We are expecting an MP3 file
//! let mp3_file = MPEGFile::read_from(&mut file_content, ParseOptions::new())?;
//!
//! assert_eq!(mp3_file.properties().channels(), 2);
//!
//! // Here we have a file with multiple tags
//! assert!(mp3_file.contains_tag_type(TagType::ID3v2));
//! assert!(mp3_file.contains_tag_type(TagType::APE));
//! # Ok(())
//! # }
//! ```
//!
//! # Features
//!
//! ## Individual metadata formats
//! These features are available if you have a specific use case, or just don't want certain formats.
//!
//! * `aiff_text_chunks`
//! * `ape`
//! * `id3v1`
//! * `id3v2`
//! * `mp4_ilst`
//! * `riff_info_list`
//! * `vorbis_comments`
//!
//! ## Utilities
//! * `id3v2_restrictions` - Parses ID3v2 extended headers and exposes flags for fine grained control
//!
//! # Important format-specific notes
//!
//! All formats have their own quirks that may produce unexpected results between conversions.
//! Be sure to read the module documentation of each format to see important notes and warnings.
#![forbid(clippy::dbg_macro, clippy::string_to_string)]
#![deny(
	clippy::pedantic,
	clippy::all,
	missing_docs,
	rustdoc::broken_intra_doc_links,
	rust_2018_idioms,
	trivial_casts,
	trivial_numeric_casts,
	unused_import_braces,
	explicit_outlives_requirements
)]
// TODO: This had multiple FPs right now, remove this when it is fixed
#![allow(clippy::needless_borrow)]
// TODO: Remove this when Ubuntu has 1.62.0 in its stable repos
#![allow(clippy::unnecessary_lazy_evaluations)]
#![allow(
	unknown_lints,
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
	clippy::new_without_default,
	clippy::from_over_into,
	clippy::upper_case_acronyms,
	clippy::single_match_else,
	clippy::similar_names,
	clippy::tabs_in_doc_comments,
	clippy::len_without_is_empty,
	clippy::needless_late_init,
	clippy::type_complexity,
	clippy::type_repetition_in_bounds,
	unused_qualifications,
	clippy::return_self_not_must_use,
	clippy::bool_to_int_with_if
)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

// TODO: Give 1.62.0 some time, and start using #[default] on enums

// proc macro hacks
extern crate self as lofty;

pub mod ape;
pub mod error;
pub(crate) mod file;
pub mod flac;
pub mod id3;
pub mod iff;
pub(crate) mod macros;
pub mod mp4;
pub mod mpeg;
pub mod ogg;
pub(crate) mod picture;
mod probe;
pub(crate) mod properties;
pub mod resolve;
pub(crate) mod tag;
mod traits;
mod util;
pub mod wavpack;

pub use crate::error::{LoftyError, Result};

pub use crate::probe::{read_from, read_from_path, ParseOptions, ParsingMode, Probe};

pub use crate::file::{AudioFile, FileType, TaggedFile};
pub use crate::picture::{MimeType, Picture, PictureType};
pub use crate::properties::FileProperties;
pub use crate::tag::{Tag, TagType};
pub use tag::item::{ItemKey, ItemValue, TagItem};

pub use crate::traits::{Accessor, TagExt};

#[cfg(feature = "vorbis_comments")]
pub use picture::PictureInformation;

// TODO: https://github.com/rust-lang/rust/issues/88581
#[inline]
pub(crate) const fn div_ceil(dividend: u64, divisor: u64) -> u64 {
	let d = dividend / divisor;
	let r = dividend % divisor;
	if r > 0 && divisor > 0 {
		d + 1
	} else {
		d
	}
}
