use crate::error::{ErrorKind, LoftyError, Result};
#[cfg(feature = "id3v2")]
use crate::id3::v2;
#[allow(unused_imports)]
use crate::types::tag::{Tag, TagType};

use std::fs::File;

#[allow(unused_variables)]
pub(crate) fn write_to(data: &mut File, tag: &Tag) -> Result<()> {
	match tag.tag_type() {
		#[cfg(feature = "riff_info_list")]
		TagType::RiffInfo => {
			super::tag::RiffInfoListRef::new(super::tag::tagitems_into_riff(tag.items()))
				.write_to(data)
		},
		#[cfg(feature = "id3v2")]
		TagType::Id3v2 => v2::tag::Id3v2TagRef {
			flags: v2::Id3v2TagFlags::default(),
			frames: v2::tag::tag_frames(tag),
		}
		.write_to(data),
		_ => Err(LoftyError::new(ErrorKind::UnsupportedTag)),
	}
}
