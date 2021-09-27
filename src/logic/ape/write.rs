use crate::error::{LoftyError, Result};
use crate::types::tag::{Tag, TagType};

use std::fs::File;

pub(crate) fn write_to(data: &mut File, tag: &Tag) -> Result<()> {
	match tag.tag_type() {
		TagType::Ape => super::tag::write::write_to(data, tag),
		TagType::Id3v1 => crate::logic::id3::v1::write::write_id3v1(data, tag),
		_ => Err(LoftyError::UnsupportedTag),
	}
}
