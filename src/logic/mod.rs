pub(crate) mod ape;
pub(crate) mod iff;
pub(crate) mod mp4;
pub(crate) mod mpeg;
pub(crate) mod ogg;
use ogg::constants::{OPUSTAGS, VORBIS_COMMENT_HEAD};

#[cfg(any(feature = "id3v1", feature = "id3v2"))]
pub(crate) mod id3;

use crate::error::Result;
use crate::types::file::FileType;
use crate::types::tag::Tag;

use std::fs::File;

pub(crate) fn write_tag(tag: &Tag, file: &mut File, file_type: FileType) -> Result<()> {
	match file_type {
		FileType::AIFF => iff::aiff::write::write_to(file, tag),
		FileType::APE => ape::write::write_to(file, tag),
		FileType::FLAC => ogg::flac::write::write_to(file, tag),
		FileType::MP3 => mpeg::write::write_to(file, tag),
		FileType::MP4 => mp4::ilst::write::write_to(file, tag),
		FileType::Opus => ogg::write::write_to(file, tag, OPUSTAGS),
		FileType::Vorbis => ogg::write::write_to(file, tag, VORBIS_COMMENT_HEAD),
		FileType::WAV => iff::wav::write::write_to(file, tag),
	}
}
