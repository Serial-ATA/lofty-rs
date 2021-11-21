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
//! use lofty::files::Mp3File;
//! use lofty::files::AudioFile;
//! use lofty::TagType;
//! use std::fs::File;
//!
//! let mut file_content = File::open("tests/assets/a.mp3").unwrap();
//!
//! let mpeg_file = Mp3File::read_from(&mut file_content).unwrap();
//!
//! assert_eq!(mpeg_file.properties().channels(), 2);
//!
//! // Here we have a file with multiple tags
//! assert!(mpeg_file.contains_tag_type(&TagType::Id3v2));
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
//! assert_eq!(tagged_file.file_type(), &FileType::MP3);
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
//!
//! ## Utilities
//! * `id3v2_restrictions` - Parses ID3v2 extended headers and exposes flags for fine grained control
//!
//! # Notes on ID3
//!
//! See [`id3`](crate::id3) for important warnings and notes on reading tags.

#![deny(clippy::pedantic, clippy::all)]
// TODO missing_docs
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
	clippy::upper_case_acronyms
)]

pub use crate::error::{LoftyError, Result};

pub use crate::probe::Probe;

pub use crate::types::{
	file::{FileType, TaggedFile},
	item::{ItemKey, ItemValue, TagItem},
	properties::FileProperties,
	tag::{Tag, TagType},
};

mod types;

/// Various concrete file types, used when inference is unnecessary
pub mod files {
	pub use crate::logic::ape::{ApeFile, ApeProperties};
	pub use crate::logic::iff::{
		aiff::AiffFile,
		wav::{
			properties::{WavFormat, WavProperties},
			WavFile,
		},
	};
	pub use crate::logic::mp3::{
		header::{ChannelMode, Layer, MpegVersion},
		Mp3File, Mp3Properties,
	};
	pub use crate::logic::mp4::{Mp4Codec, Mp4File, Mp4Properties};
	pub use crate::logic::ogg::{
		flac::FlacFile,
		opus::{properties::OpusProperties, OpusFile},
		vorbis::{properties::VorbisProperties, VorbisFile},
	};
	pub use crate::types::file::AudioFile;
}

/// Various concrete tag types, used when format-specific features are necessary
pub mod tags {
	pub use crate::logic::id3::v1::tag::Id3v1Tag;
	pub use crate::logic::iff::{aiff::tag::AiffTextChunks, wav::tag::RiffInfoList};
	pub use crate::logic::ogg::tag::VorbisComments;
}

#[cfg(any(feature = "id3v1", feature = "id3v2"))]
/// ID3v1/v2 specific items
pub mod id3 {
	//! ID3 does things differently than other tags, making working with them a little more effort than other formats.
	//! Check the other modules for important notes and/or warnings.

	#[cfg(feature = "id3v2")]
	pub mod v2 {
		//! ID3v2 items and utilities
		//!
		//! # ID3v2 notes and warnings
		// TODO

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
		//! ## Genres
		//!
		//! ID3v1 stores the genre in a single byte ranging from 0 to 192.
		//! The number can be stored in any of the following [`ItemValue`](crate::ItemValue) variants: `Text, UInt, UInt64, Int, Int64`, and will be discarded if it is unable to parse or is too big.
		//! All possible genres have been stored in the [`GENRES`](crate::id3::v1::GENRES) constant.
		//!
		//! ## Track Numbers
		//!
		//! ID3v1 stores the track number in a non-zero byte.
		//! A track number of 0 will be treated as an empty field.
		//! Additionally, there is no track total field.

		pub use crate::logic::id3::v1::constants::GENRES;
	}
}

/// MP4 specific items
pub mod mp4 {
	pub use crate::logic::mp4::ilst::{Atom, AtomData, AtomIdent};
}

/// Various items related to [`Picture`](crate::picture::Picture)s
pub mod picture {
	pub use crate::types::picture::{MimeType, Picture, PictureInformation, PictureType};
}

mod error;
pub(crate) mod logic;
mod probe;
