use super::read::verify_wav;
use crate::error::{LoftyError, Result};
use crate::logic::id3::v2::tag::Id3v2TagRef;
use crate::logic::iff::wav::tag::RiffInfoListRef;
use crate::types::tag::{Tag, TagType};

use std::fs::File;

use byteorder::LittleEndian;

pub(crate) fn write_to(data: &mut File, tag: &Tag) -> Result<()> {
	verify_wav(data)?;

	match tag.tag_type() {
		TagType::RiffInfo => Into::<RiffInfoListRef>::into(tag).write_to(data),
		TagType::Id3v2 => Into::<Id3v2TagRef>::into(tag).write_to_chunk_file::<LittleEndian>(data),
		_ => Err(LoftyError::UnsupportedTag),
	}
}
