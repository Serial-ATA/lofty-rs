use super::read::verify_wav;
use crate::error::{LoftyError, Result};
use crate::types::tag::{Tag, TagType};

use std::fs::File;

pub(crate) fn write_to(data: &mut File, tag: &Tag) -> Result<()> {
	verify_wav(data)?;

	match tag.tag_type() {
		TagType::RiffInfo => super::tag::write::write_riff_info(data, tag),
		TagType::Id3v2 => crate::logic::id3::v2::write::write_id3v2_to_chunk_file::<
			byteorder::LittleEndian,
		>(data, tag),
		_ => Err(LoftyError::UnsupportedTag),
	}
}
