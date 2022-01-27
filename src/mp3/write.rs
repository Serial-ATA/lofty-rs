#[cfg(feature = "ape")]
use crate::ape::tag::ape_tag;
use crate::error::{ErrorKind, LoftyError, Result};
#[cfg(feature = "id3v1")]
use crate::id3::v1;
#[cfg(feature = "id3v2")]
use crate::id3::v2;
#[allow(unused_imports)]
use crate::types::tag::{Tag, TagType};

use std::fs::File;

#[allow(unused_variables)]
pub(crate) fn write_to(data: &mut File, tag: &Tag) -> Result<()> {
	match tag.tag_type() {
		#[cfg(feature = "ape")]
		TagType::Ape => Into::<ape_tag::ApeTagRef>::into(tag).write_to(data),
		#[cfg(feature = "id3v1")]
		TagType::Id3v1 => Into::<v1::tag::Id3v1TagRef>::into(tag).write_to(data),
		#[cfg(feature = "id3v2")]
		TagType::Id3v2 => {
			v2::tag::Id3v2TagRef::new(v2::Id3v2TagFlags::default(), v2::tag::tag_frames(tag))
				.write_to(data)
		},
		_ => Err(LoftyError::new(ErrorKind::UnsupportedTag)),
	}
}
