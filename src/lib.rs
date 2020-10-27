//! # audiotags
//!
//! [![Crate](https://img.shields.io/crates/v/audiotags.svg)](https://crates.io/crates/audiotags)
//! [![Crate](https://img.shields.io/crates/d/audiotags.svg)](https://crates.io/crates/audiotags)
//! [![Crate](https://img.shields.io/crates/l/audiotags.svg)](https://crates.io/crates/audiotags)
//! [![Documentation](https://docs.rs/audiotags/badge.svg)](https://docs.rs/audiotags/)
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
//! Using **audiotags** incurs a little overhead due to vtables if you want to guess the metadata format
//! (from file extension). Apart from this the performance is almost the same as directly calling function
//! provided by those 'specialized' crates.
//!
//! No copies will be made if you only need to read and write metadata of one format. If you want to convert
//! between tags, copying is unavoidable no matter if you use **audiotags** or use getters and setters provided
//! by specialized libraries. **audiotags** is not making additional unnecessary copies.
//!
//! Theoretically it is possible to achieve zero-copy conversions if all parsers can parse into a unified
//! struct. However, this is going to be a lot of work. I might be able to implement them, but it will be no
//! sooner than the Christmas vacation.
//!
//! See [README](https://github.com/TianyiShi2001/audiotags) for some examples.

pub(crate) use audiotags_dev_macro::*;

pub mod anytag;
pub use anytag::*;

pub mod components;
pub use components::*;

pub mod error;
pub use error::{Error, Result};

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

#[derive(Clone, Copy, Debug)]
pub enum TagType {
    // /// Guess the tag type based on the file extension
    // Guess,
    /// ## Common file extensions
    ///
    /// `.mp3`
    ///
    /// ## References
    ///
    /// - https://www.wikiwand.com/en/ID3
    Id3v2,
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

#[rustfmt::skip]
impl TagType {
    fn try_from_ext(ext: &str) -> crate::Result<Self> {
        match ext {
                                                     "mp3" => Ok(Self::Id3v2),
            "m4a" | "m4b" | "m4p" | "m4v" | "isom" | "mp4" => Ok(Self::Mp4),
                                                    "flac" => Ok(Self::Flac),
            p @ _ => Err(crate::Error::UnsupportedFormat(p.to_owned())),
        }
    }
}

#[derive(Default)]
pub struct Tag {
    tag_type: Option<TagType>,
    config: Config,
}

impl Tag {
    pub fn with_tag_type(tag_type: TagType) -> Self {
        Self {
            tag_type: Some(tag_type),
            config: Config::default(),
        }
    }
    pub fn with_config(config: Config) -> Self {
        Self {
            tag_type: None,
            config: config.clone(),
        }
    }
    pub fn with_tag_type_and_config(tag_type: TagType, config: Config) -> Self {
        Self {
            tag_type: Some(tag_type),
            config: config.clone(),
        }
    }

    pub fn read_from_path(&self, path: impl AsRef<Path>) -> crate::Result<Box<dyn AudioTag>> {
        match self.tag_type.unwrap_or(TagType::try_from_ext(
            path.as_ref()
                .extension()
                .unwrap()
                .to_string_lossy()
                .to_string()
                .to_lowercase()
                .as_str(),
        )?) {
            TagType::Id3v2 => Ok(Box::new({
                let mut t = Id3v2Tag::read_from_path(path)?;
                t.set_config(self.config.clone());
                t
            })),
            TagType::Mp4 => Ok(Box::new({
                let mut t = Mp4Tag::read_from_path(path)?;
                t.set_config(self.config.clone());
                t
            })),
            TagType::Flac => Ok(Box::new({
                let mut t = FlacTag::read_from_path(path)?;
                t.set_config(self.config.clone());
                t
            })),
        }
    }
}
