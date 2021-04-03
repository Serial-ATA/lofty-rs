//! # lofty
//!
//! [![Crate](https://img.shields.io/crates/v/lofty.svg)](https://crates.io/crates/lofty)
//! [![Crate](https://img.shields.io/crates/d/lofty.svg)](https://crates.io/crates/lofty)
//! [![Crate](https://img.shields.io/crates/l/lofty.svg)](https://crates.io/crates/lofty)
//! [![Documentation](https://docs.rs/lofty/badge.svg)](https://docs.rs/lofty/)
//!
//! This crate makes it easier to parse, convert and write metadata (a.k.a tag) in audio files of different file types.
//!
//! This crate aims to provide a unified trait for parsers and writers of different audio file formats.
//! This means that you can parse tags in mp3, flac, and m4a files with a single function: `Tag::default().
//! read_from_path()` and get fields by directly calling `.album()`, `.artist()` on its result. Without this
//! crate, you would otherwise need to learn different APIs in **id3**, **mp4ameta** etc. in order to parse
//! metadata in different file formats.
//!
//! ## Performance
//!
//! Using **lofty** incurs a little overhead due to vtables if you want to guess the metadata format
//! (from file extension). Apart from this the performance is almost the same as directly calling function
//! provided by those 'specialized' crates.
//!
//! No copies will be made if you only need to read and write metadata of one format. If you want to convert
//! between tags, copying is unavoidable no matter if you use **lofty** or use getters and setters provided
//! by specialized libraries. **lofty** is not making additional unnecessary copies.
//!
//! Theoretically it is possible to achieve zero-copy conversions if all parsers can parse into a unified
//! struct. However, this is going to be a lot of work. I might be able to implement them, but it will be no
//! sooner than the Christmas vacation.
//!
//! Read the [manual](https://tianyishi2001.github.io/audiotags) for some examples.

//#![forbid(unused_crate_dependencies, unused_import_braces)]
#![warn(clippy::pedantic)]
#![allow(
	clippy::too_many_lines,
	clippy::cast_precision_loss,
	clippy::cast_sign_loss,
	clippy::cast_possible_wrap,
	clippy::clippy::cast_possible_truncation,
	clippy::module_name_repetitions
)]

mod macros;
pub use macros::*;

pub mod anytag;
pub use anytag::*;

pub mod components;
pub use components::*;

pub mod error;
pub use error::{Result, TaggedError};

pub mod traits;
pub use traits::*;

pub mod types;
pub use types::*;

pub mod config;

pub use config::Config;

use std::convert::From;
use std::fs::File;
use std::path::Path;

pub use std::convert::{TryFrom, TryInto};

/// A builder for `Box<dyn AudioTag>`. If you do not want a trait object, you can use individual types.
///
/// # Examples
///
/// ```
/// use lofty::{Tag, TagType};
///
/// // Guess the format from the extension, in this case `mp3`
/// let mut tag = Tag::new().read_from_path("assets/a.mp3").unwrap();
/// tag.set_title("Foo");
/// // You can convert the tag type and save the metadata to another file.
/// tag.to_dyn_tag(TagType::Mp4).write_to_path("assets/a.m4a");
/// // You can specify the tag type (but when you want to do this, also consider directly using the concrete type)
/// let tag = Tag::new().with_tag_type(TagType::Mp4).read_from_path("assets/a.m4a").unwrap();
/// assert_eq!(tag.title(), Some("Foo"));
/// ```

#[derive(Default)]
pub struct Tag(Option<TagType>);

impl Tag {
	/// Initiate a new Tag (a builder for `Box<dyn AudioTag>`).
	/// You can then optionally chain `with_tag_type`.
	/// Finally, you `read_from_path`.
	pub fn new() -> Self {
		Self::default()
	}
	/// Specify the TagType
	pub fn with_tag_type(self, tag_type: TagType) -> Self {
		Self(Some(tag_type))
	}
	/// Path of the file to read
	pub fn read_from_path(&self, path: impl AsRef<Path>) -> crate::Result<Box<dyn AudioTag>> {
		let extension = path.as_ref().extension().unwrap().to_str().unwrap();

		match self.0.unwrap_or(TagType::try_from_ext(extension)?) {
			TagType::Id3v2 => Ok(Box::new(Id3v2Tag::read_from_path(path)?)),
			TagType::Vorbis => Ok(Box::new(VorbisTag::read_from_path(path)?)),
			TagType::Opus => Ok(Box::new(OpusTag::read_from_path(path)?)),
			TagType::Flac => Ok(Box::new(FlacTag::read_from_path(path)?)),
			TagType::Mp4 => Ok(Box::new(Mp4Tag::read_from_path(path)?)),
		}
	}
}

#[derive(Clone, Copy, Debug)]
pub enum TagType {
	/// ## Common file extensions
	///
	/// `.mp3`
	///
	/// ## References
	///
	/// - https://www.wikiwand.com/en/ID3
	Id3v2,
	Vorbis,
	Opus,
	Flac,
	/// ## Common file extensions
	///
	/// `.mp4, .m4a, .m4p, .m4b, .m4r and .m4v`
	///
	/// ## References
	///
	/// - https://www.wikiwand.com/en/MPEG-4_Part_14
	Mp4,
}

impl TagType {
	fn try_from_ext(ext: &str) -> crate::Result<Self> {
		match ext {
			"mp3" => Ok(Self::Id3v2),
			"ogg" => Ok(Self::Vorbis),
			"opus" => Ok(Self::Opus),
			"flac" => Ok(Self::Flac),
			"m4a" | "m4b" | "m4p" | "m4v" | "isom" | "mp4" => Ok(Self::Mp4),
			_ => Err(TaggedError::UnsupportedFormat(ext.to_owned())),
		}
	}
}

/// Convert a concrete tag type into another
#[macro_export]
macro_rules! convert {
	($inp:expr, $target_type:ty) => {
		$target_type::from(inp.to_anytag())
	};
}
