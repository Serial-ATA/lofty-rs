//! [![GitHub Workflow Status](https://img.shields.io/github/workflow/status/Serial-ATA/lofty-rs/CI?style=for-the-badge&logo=github)](https://github.com/Serial-ATA/lofty-rs/actions/workflows/ci.yml)
//! [![Downloads](https://img.shields.io/crates/d/lofty?style=for-the-badge&logo=rust)](https://crates.io/crates/lofty)
//! [![Version](https://img.shields.io/crates/v/lofty?style=for-the-badge&logo=rust)](https://crates.io/crates/lofty)
//!
//! Parse, convert, and write metadata to audio formats.
//!
//! # Supported Formats
//!
//! | File Format | Extensions                                      | Read | Write | Metadata Format(s)                            |
//! |-------------|-------------------------------------------------|------|-------|-----------------------------------------------|
//! | APE         | `ape`                                           |**X** |**X**  |`APEv2`, `APEv1`, `ID3v2` (Read only), `ID3v1` |
//! | AIFF        | `aiff`, `aif`                                   |**X** |**X**  |`ID3v2`, `Text Chunks`                         |
//! | FLAC        | `flac`                                          |**X** |**X**  |`Vorbis Comments`                              |
//! | MP3         | `mp3`                                           |**X** |**X**  |`ID3v2`, `ID3v1`, `APEv2`, `APEv1`             |
//! | MP4         | `mp4`, `m4a`, `m4b`, `m4p`, `m4r`, `m4v`, `3gp` |**X** |**X**  |`iTunes-style ilst`                            |
//! | Opus        | `opus`                                          |**X** |**X**  |`Vorbis Comments`                              |
//! | Ogg Vorbis  | `ogg`                                           |**X** |**X**  |`Vorbis Comments`                              |
//! | WAV         | `wav`, `wave`                                   |**X** |**X**  |`ID3v2`, `RIFF INFO`                           |
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
//! ```rust
//! # use lofty::LoftyError;
//! # fn main() -> Result<(), LoftyError> {
//! use lofty::{read_from_path, Probe};
//!
//! // First, create a probe.
//! // This will guess the format from the extension
//! // ("mp3" in this case), but we can guess from the content if we want to.
//! let tagged_file = read_from_path("tests/files/assets/a.mp3")?;
//!
//! // Let's guess the format from the content just in case.
//! // This is not necessary in this case!
//! let tagged_file2 = Probe::open("tests/files/assets/a.mp3")?.guess_file_type()?.read()?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Using an existing reader
//!
//! ```rust
//! # use lofty::LoftyError;
//! # fn main() -> Result<(), LoftyError> {
//! use std::fs::File;
//! use lofty::read_from;
//!
//! // Let's read from an open file
//! let mut file = File::open("tests/files/assets/a.mp3")?;
//!
//! // Here, we have to guess the file type prior to reading
//! let tagged_file = read_from(&mut file)?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Accessing tags
//!
//! ```rust
//! # use lofty::LoftyError;
//! # fn main() -> Result<(), LoftyError> {
//! use lofty::read_from_path;
//!
//! let tagged_file = read_from_path("tests/files/assets/a.mp3")?;
//!
//! // Get the primary tag (ID3v2 in this case)
//! let id3v2 = tagged_file.primary_tag().unwrap();
//!
//! // If the primary tag doesn't exist, or the tag types
//! // don't matter, the first tag can be retrieved
//! let unknown_first_tag = tagged_file.first_tag().unwrap();
//! # Ok(())
//! # }
//! ```
//!
//! ## Using concrete file types
//!
//! ```rust
//! # use lofty::LoftyError;
//! # fn main() -> Result<(), LoftyError> {
//! use lofty::mp3::Mp3File;
//! use lofty::AudioFile;
//! use lofty::TagType;
//! use std::fs::File;
//!
//! let mut file_content = File::open("tests/files/assets/a.mp3")?;
//!
//! // We are expecting an MP3 file
//! let mpeg_file = Mp3File::read_from(&mut file_content)?;
//!
//! assert_eq!(mpeg_file.properties().channels(), 2);
//!
//! // Here we have a file with multiple tags
//! assert!(mpeg_file.contains_tag_type(&TagType::Id3v2));
//! assert!(mpeg_file.contains_tag_type(&TagType::Ape));
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
#![deny(
	clippy::pedantic,
	clippy::all,
	missing_docs,
	rustdoc::broken_intra_doc_links
)]
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
	clippy::used_underscore_binding,
	clippy::new_without_default,
	clippy::unused_self,
	clippy::from_over_into,
	clippy::upper_case_acronyms,
	clippy::too_many_arguments,
	clippy::single_match_else
)]

mod error;
pub(crate) mod logic;
mod probe;
mod types;

pub use crate::error::{LoftyError, Result};

pub use crate::probe::Probe;

pub use crate::types::{
	file::{FileType, TaggedFile},
	item::{ItemKey, ItemValue, TagItem},
	properties::FileProperties,
	tag::{Tag, TagType},
};

pub use crate::types::file::AudioFile;

pub use crate::types::picture::{MimeType, Picture, PictureInformation, PictureType};

pub use probe::{read_from, read_from_path};

#[cfg(any(feature = "id3v1", feature = "id3v2"))]
pub mod id3 {
	//! ID3 specific items
	//!
	//! ID3 does things differently than other tags, making working with them a little more effort than other formats.
	//! Check the other modules for important notes and/or warnings.

	#[cfg(feature = "id3v2")]
	pub mod v2 {
		//! ID3v2 items and utilities
		//!
		//! ## Important notes
		//!
		//! See:
		//!
		//! * [Id3v2Tag]
		//! * [Frame]

		pub use {
			crate::logic::id3::v2::frame::{
				EncodedTextFrame, Frame, FrameFlags, FrameID, FrameValue, LanguageFrame,
			},
			crate::logic::id3::v2::items::encapsulated_object::{
				GEOBInformation, GeneralEncapsulatedObject,
			},
			crate::logic::id3::v2::items::sync_text::{
				SyncTextContentType, SyncTextInformation, SynchronizedText, TimestampFormat,
			},
			crate::logic::id3::v2::tag::{Id3v2Tag, Id3v2TagFlags},
			crate::logic::id3::v2::util::text_utils::TextEncoding,
			crate::logic::id3::v2::util::upgrade::{upgrade_v2, upgrade_v3},
			crate::logic::id3::v2::Id3v2Version,
		};

		#[cfg(feature = "id3v2_restrictions")]
		pub use crate::logic::id3::v2::items::restrictions::*;
	}

	#[cfg(feature = "id3v1")]
	pub mod v1 {
		//! ID3v1 items
		//!
		//! # ID3v1 notes
		//!
		//! See also: [Id3v1Tag]
		//!
		//! ## Genres
		//!
		//! ID3v1 stores the genre in a single byte ranging from 0 to 192 (inclusive).
		//! All possible genres have been stored in the [`GENRES`] constant.
		//!
		//! ## Track Numbers
		//!
		//! ID3v1 stores the track number in a non-zero byte.
		//! A track number of 0 will be treated as an empty field.
		//! Additionally, there is no track total field.
		pub use crate::logic::id3::v1::constants::GENRES;
		pub use crate::logic::id3::v1::tag::Id3v1Tag;
	}
}

pub mod ape {
	//! APE specific items
	//!
	//! ## File notes
	//!
	//! It is possible for an `APE` file to contain an `ID3v2` tag. For the sake of data preservation,
	//! this tag will be read, but **cannot** be written. The only tags allowed by spec are `APEv1/2` and
	//! `ID3v1`.
	#[cfg(feature = "ape")]
	pub use crate::logic::ape::tag::item::ApeItem;
	#[cfg(feature = "ape")]
	pub use crate::logic::ape::tag::ApeTag;
	pub use crate::logic::ape::{ApeFile, ApeProperties};
	pub use crate::types::picture::APE_PICTURE_TYPES;
}

pub mod mp3 {
	//! MP3 specific items
	pub use crate::logic::mp3::header::{ChannelMode, Layer, MpegVersion};
	pub use crate::logic::mp3::{Mp3File, Mp3Properties};
}

pub mod mp4 {
	//! MP4 specific items
	//!
	//! ## File notes
	//!
	//! The only supported tag format is [`Ilst`].
	#[cfg(feature = "mp4_ilst")]
	pub use crate::logic::mp4::{
		ilst::{
			atom::{Atom, AtomData},
			Ilst,
		},
		AtomIdent,
	};
	pub use crate::logic::mp4::{Mp4Codec, Mp4File, Mp4Properties};
}

pub mod ogg {
	//! OPUS/FLAC/Vorbis specific items
	//!
	//! ## File notes
	//!
	//! The only supported tag format is [`VorbisComments`]
	pub use crate::logic::ogg::flac::FlacFile;
	pub use crate::logic::ogg::opus::{properties::OpusProperties, OpusFile};
	#[cfg(feature = "vorbis_comments")]
	pub use crate::logic::ogg::tag::VorbisComments;
	pub use crate::logic::ogg::vorbis::{properties::VorbisProperties, VorbisFile};
}

pub mod iff {
	//! WAV/AIFF specific items
	pub use crate::logic::iff::aiff::AiffFile;
	pub use crate::logic::iff::wav::WavFile;

	#[cfg(feature = "aiff_text_chunks")]
	pub use crate::logic::iff::aiff::tag::AiffTextChunks;
	#[cfg(feature = "riff_info_list")]
	pub use crate::logic::iff::wav::tag::RiffInfoList;

	pub use crate::logic::iff::wav::properties::{WavFormat, WavProperties};
}
