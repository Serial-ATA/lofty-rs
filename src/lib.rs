//! [![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/Serial-ATA/lofty-rs/ci.yml?branch=main&logo=github&style=for-the-badge)](https://github.com/Serial-ATA/lofty-rs/actions/workflows/ci.yml)
//! [![Downloads](https://img.shields.io/crates/d/lofty?style=for-the-badge&logo=rust)](https://crates.io/crates/lofty)
//! [![Version](https://img.shields.io/crates/v/lofty?style=for-the-badge&logo=rust)](https://crates.io/crates/lofty)
//!
//! Parse, convert, and write metadata to audio formats.
//!
//! # Supported Formats
#![doc = include_str!("../SUPPORTED_FORMATS.md")]
//! # Examples
//!
//! ## Reading a generic file
//!
//! It isn't always convenient to [use concrete file types](#using-concrete-file-types), which is where [`TaggedFile`](file::TaggedFile)
//! comes in.
//!
//! ### Using a path
//!
//! ```rust,no_run
//! # fn main() -> lofty::error::Result<()> {
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
//! # fn main() -> lofty::error::Result<()> {
//! use lofty::config::ParseOptions;
//! use lofty::read_from;
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
//! # fn main() -> lofty::error::Result<()> {
//! use lofty::file::TaggedFileExt;
//! use lofty::read_from_path;
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
//! # fn main() -> lofty::error::Result<()> {
//! use lofty::config::ParseOptions;
//! use lofty::file::AudioFile;
//! use lofty::mpeg::MpegFile;
//! use lofty::tag::TagType;
//! use std::fs::File;
//!
//! # let path = "tests/files/assets/minimal/full_test.mp3";
//! let mut file_content = File::open(path)?;
//!
//! // We are expecting an MP3 file
//! let mp3_file = MpegFile::read_from(&mut file_content, ParseOptions::new())?;
//!
//! assert_eq!(mp3_file.properties().channels(), 2);
//!
//! // Here we have a file with multiple tags
//! assert!(mp3_file.contains_tag_type(TagType::Id3v2));
//! assert!(mp3_file.contains_tag_type(TagType::Ape));
//! # Ok(())
//! # }
//! ```
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
	clippy::match_wildcard_for_single_variants,
	clippy::semicolon_if_nothing_returned,
	clippy::from_over_into,
	clippy::upper_case_acronyms,
	clippy::single_match_else,
	clippy::similar_names,
	clippy::tabs_in_doc_comments,
	clippy::len_without_is_empty,
	clippy::needless_late_init,
	clippy::type_complexity,
	clippy::return_self_not_must_use,
	clippy::bool_to_int_with_if,
	clippy::uninlined_format_args, /* This should be changed for any normal "{}", but I'm not a fan of it for any debug or width specific formatting */
	clippy::let_underscore_untyped,
	clippy::field_reassign_with_default,
	clippy::manual_range_patterns, /* This is not at all clearer as it suggests */
	clippy::no_effect_underscore_binding,
	clippy::used_underscore_binding,
	clippy::ignored_unit_patterns, /* Not a fan of this lint, doesn't make anything clearer as it claims */
	clippy::needless_return, /* Explicit returns are needed from time to time for clarity */
	clippy::redundant_guards, /* Currently broken for some cases, might enable later*/
	clippy::into_iter_without_iter, /* This is only going to fire on some internal types, doesn't matter much */
)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(html_logo_url = "https://raw.githubusercontent.com/Serial-ATA/lofty-rs/main/doc/lofty.svg")]

// proc macro hacks
extern crate self as lofty;
pub(crate) mod _this_is_internal {}

pub mod config;
pub mod error;
pub mod file;
pub(crate) mod macros;
mod math;
pub(crate) mod picture;
mod probe;
pub mod properties;
pub mod resolve;
pub mod tag;
mod traits;
mod util;

pub mod aac;
pub mod ape;
pub mod flac;
pub mod id3;
pub mod iff;
pub mod mp4;
pub mod mpeg;
pub mod musepack;
pub mod ogg;
pub mod wavpack;

pub use crate::probe::{read_from, read_from_path, Probe};

pub use crate::picture::{MimeType, Picture, PictureType};
pub use util::text::TextEncoding;

pub use picture::PictureInformation;

pub use lofty_attr::LoftyFile;

pub mod prelude {
	//! A prelude for commonly used items in the library.
	//!
	//! This module is intended to be wildcard imported.
	//!
	//! ```rust
	//! use lofty::prelude::*;
	//! ```

	pub use crate::error::LoftyError;
	pub use crate::file::{AudioFile, TaggedFileExt};
	pub use crate::tag::{Accessor, ItemKey, MergeTag, SplitTag, TagExt};
}
