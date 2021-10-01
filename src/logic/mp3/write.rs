use crate::error::{LoftyError, Result};
use crate::types::tag::{Tag, TagType};

use std::fs::File;

pub(crate) fn write_to(data: &mut File, tag: &Tag) -> Result<()> {
	match tag.tag_type() {
		TagType::Ape => crate::logic::ape::tag::write::write_to(data, tag),
		TagType::Id3v1 => crate::logic::id3::v1::write::write_id3v1(data, tag),
		TagType::Id3v2 => crate::logic::id3::v2::write::write_id3v2(data, tag),
		_ => Err(LoftyError::UnsupportedTag),
	}
}
