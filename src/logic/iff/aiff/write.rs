use crate::error::{LoftyError, Result};
use crate::logic::id3::v2::tag::Id3v2TagRef;
use crate::logic::iff::aiff::tag::AiffTextChunksRef;
use crate::types::tag::{Tag, TagType};

use std::fs::File;

use byteorder::BigEndian;

pub(crate) fn write_to(data: &mut File, tag: &Tag) -> Result<()> {
	match tag.tag_type() {
		#[cfg(feature = "aiff_text_chunks")]
		TagType::AiffText => Into::<AiffTextChunksRef>::into(tag).write_to(data),
		#[cfg(feature = "id3v2")]
		TagType::Id3v2 => Into::<Id3v2TagRef>::into(tag).write_to_chunk_file::<BigEndian>(data),
		_ => Err(LoftyError::UnsupportedTag),
	}
}
