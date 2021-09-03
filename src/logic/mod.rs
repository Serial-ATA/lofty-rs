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
		FileType::AIFF => Ok(()), // TODO
		FileType::APE => Ok(()),  // TODO
		FileType::FLAC => Ok(()), // TODO
		FileType::MP3 => Ok(()),  // TODO
		FileType::MP4 => Ok(()),  // TODO
		FileType::Opus => ogg::write::create_pages(file, OPUSTAGS, tag),
		FileType::Vorbis => ogg::write::create_pages(file, VORBIS_COMMENT_HEAD, tag),
		FileType::WAV => Ok(()), // TODO
	}
}
