use crate::error::{LoftyError, Result};
use crate::logic::ape::tag::ApeTagRef;
use crate::logic::id3::v1::tag::Id3v1TagRef;
use crate::types::tag::{Tag, TagType};

use std::fs::File;

pub(in crate::logic) fn write_to(data: &mut File, tag: &Tag) -> Result<()> {
	match tag.tag_type() {
		TagType::Ape => Into::<ApeTagRef>::into(tag).write_to(data),
		TagType::Id3v1 => Into::<Id3v1TagRef>::into(tag).write_to(data),
		_ => Err(LoftyError::UnsupportedTag),
	}
}
