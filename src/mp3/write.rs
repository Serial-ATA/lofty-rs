#[cfg(feature = "ape")]
use crate::ape;
use crate::error::{ErrorKind, LoftyError, Result};
#[cfg(feature = "id3v1")]
use crate::id3::v1;
#[cfg(feature = "id3v2")]
use crate::id3::v2;
#[allow(unused_imports)]
use crate::tag::{Tag, TagType};

use std::fs::File;

#[allow(unused_variables)]
pub(crate) fn write_to(data: &mut File, tag: &Tag) -> Result<()> {
	match tag.tag_type() {
		#[cfg(feature = "ape")]
		TagType::APE => ape::tag::ApeTagRef {
			read_only: false,
			items: ape::tag::tagitems_into_ape(tag.items()),
		}
		.write_to(data),
		#[cfg(feature = "id3v1")]
		TagType::ID3v1 => Into::<v1::tag::Id3v1TagRef<'_>>::into(tag).write_to(data),
		#[cfg(feature = "id3v2")]
		TagType::ID3v2 => v2::tag::Id3v2TagRef {
			flags: v2::Id3v2TagFlags::default(),
			frames: v2::tag::tag_frames(tag),
		}
		.write_to(data),
		_ => Err(LoftyError::new(ErrorKind::UnsupportedTag)),
	}
}
