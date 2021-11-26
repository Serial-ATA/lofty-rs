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
//! | MP4         | `mp4`, `m4a`, `m4b`, `m4p`, `m4r`, `m4v`, `3gp` |**X** |**X**  |`Atoms`                                        |
//! | Opus        | `opus`                                          |**X** |**X**  |`Vorbis Comments`                              |
//! | Ogg Vorbis  | `ogg`                                           |**X** |**X**  |`Vorbis Comments`                              |
//! | WAV         | `wav`, `wave`                                   |**X** |**X**  |`ID3v2`, `RIFF INFO`                           |
//!
//! # Examples
//!
//! ## Determining a file's format
//!
//! These don't read the file's properties or tags. Instead, they determine the [`FileType`], which is useful for matching
//! against [`concrete file types`](#using-concrete-file-types).
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
//! use lofty::mp3::Mp3File;
//! use lofty::AudioFile;
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
//! # Important format-specific notes
//!
//! All formats have their own quirks that may produce unexpected results between conversions.
//! Be sure to read the module documentation of each format to see important notes and warnings.
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
	clippy::upper_case_acronyms,
	clippy::too_many_arguments
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
		//! # ID3v2 notes and warnings
		//!
		//! ## Conversions
		//!
		//! ⚠ **Warnings** ⚠
		//!
		//! ### `Tag` -> `Id3v2Tag`
		//!
		//! When converting from a [`Tag`](crate::Tag) to an [`Id3v2Tag`], some frames may need editing.
		//!
		//! * [`ItemKey::Comment`](crate::ItemKey::Comment) and [`ItemKey::Lyrics`](crate::ItemKey::Lyrics) - Rather than be a normal text frame, these require a [`LanguageFrame`].
		//! An attempt is made to create this information, but it may be incorrect.
		//!    * `language` - Assumed to be "eng"
		//!    * `description` - Left empty, which is invalid if there are more than one of these frames. These frames can only be identified
		//!    by their descriptions, and as such they are expected to be unique for each.
		//! * [`ItemKey::Unknown("WXXX" | "TXXX")`](crate::ItemKey::Unknown) - These frames are also identified by their descriptions.
		//!
		//! ### `Id3v2Tag` -> `Tag`
		//!
		//! Converting an [`Id3v2Tag`] to a [`Tag`](crate::Tag) will not retain any frame-specific information, due
		//! to ID3v2 being the only format that requires such information. This includes things like [`TextEncoding`] and [`LanguageFrame`].
		//!
		//! ## Special Frames
		//!
		//! ID3v2 has `GEOB` and `SYLT` frames, which are not parsed by default, instead storing them as [`FrameValue::Binary`].
		//! They can easily be parsed with [`GeneralEncapsulatedObject::parse`] and [`SynchronizedText::parse`] respectively, and converted
		//! back to binary with [`GeneralEncapsulatedObject::as_bytes`] and [`SynchronizedText::as_bytes`] for writing.
		//!
		//! ## Outdated Frames
		//!
		//! ### ID3v2.2
		//!
		//! `ID3v2.2` frame IDs are 3 characters. When reading these tags, [`upgrade_v2`] is used, which has a list of all of the common IDs
		//! that have a mapping to `ID3v2.4`. Any ID that fails to be converted will be stored as [`FrameID::Outdated`], and it must be manually
		//! upgraded before it can be written. **Lofty** will not write `ID3v2.2` tags.
		//!
		//! ### ID3v2.3
		//!
		//! `ID3v2.3`, unlike `ID3v2.2`, stores frame IDs in 4 characters like `ID3v2.4`. There are some IDs that need upgrading (See [`upgrade_v3`]),
		//! but anything that fails to be upgraded **will not** be stored as [`FrameID::Outdated`], as it is likely not an issue to write.

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
		//! ID3v1 stores the genre in a single byte ranging from 0 to 192 (inclusive).
		//! All possible genres have been stored in the [`GENRES`] constant.
		//!
		//! ### Converting from `Tag`
		//!
		//! Two checks are performed when converting a genre:
		//!
		//! * [`GENRE`] contains the string
		//! * The [`ItemValue`](crate::ItemValue) can be parsed into a `u8`
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
	// TODO
	pub use crate::logic::ape::tag::item::ApeItem;
	pub use crate::logic::ape::tag::ApeTag;
	pub use crate::logic::ape::{ApeFile, ApeProperties};
}

pub mod mp3 {
	//! MP3 specific items
	// TODO
	pub use crate::logic::mp3::header::{ChannelMode, Layer, MpegVersion};
	pub use crate::logic::mp3::{Mp3File, Mp3Properties};
}

pub mod mp4 {
	//! MP4 specific items
	// TODO
	pub use crate::logic::mp4::ilst::{Atom, AtomData, AtomIdent, Ilst};
	pub use crate::logic::mp4::{Mp4Codec, Mp4File, Mp4Properties};
}

pub mod ogg {
	//! OPUS/FLAC/Vorbis specific items
	// TODO
	pub use crate::logic::ogg::flac::FlacFile;
	pub use crate::logic::ogg::opus::{properties::OpusProperties, OpusFile};
	pub use crate::logic::ogg::tag::VorbisComments;
	pub use crate::logic::ogg::vorbis::{properties::VorbisProperties, VorbisFile};
}

pub mod iff {
	//! WAV/AIFF specific items
	// TODO
	pub use crate::logic::iff::aiff::AiffFile;
	pub use crate::logic::iff::wav::WavFile;

	pub use crate::logic::iff::aiff::tag::AiffTextChunks;
	pub use crate::logic::iff::wav::tag::RiffInfoList;

	pub use crate::logic::iff::wav::properties::{WavFormat, WavProperties};
}
