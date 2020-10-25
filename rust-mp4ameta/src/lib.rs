//! A library for reading and writing iTunes style MPEG-4 audio metadata.
//!
//! # Example
//!
//! ```no_run
//! let mut tag = mp4ameta::Tag::read_from_path("music.m4a").unwrap();
//!
//! println!("{}", tag.artist().unwrap());
//!
//! tag.set_artist("artist");
//!
//! tag.write_to_path("music.m4a").unwrap();
//! ```
#![warn(missing_docs)]

#[macro_use]
extern crate lazy_static;

pub use crate::core::{
    atom,
    atom::Atom,
    atom::AtomT,
    atom::Ident,
    content::Content,
    content::ContentT,
    data,
    data::Data,
    data::DataT,
    types,
    types::AdvisoryRating,
    types::MediaType,
};
pub use crate::error::{
    Error,
    ErrorKind,
    Result,
};
pub use crate::tag::{
    STANDARD_GENRES,
    Tag,
};

mod core;
mod error;
mod tag;
