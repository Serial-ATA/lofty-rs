#[cfg(feature = "ape")]
use crate::ape;
use crate::error::{ErrorKind, LoftyError, Result};
#[cfg(feature = "id3v1")]
use crate::id3::v1;
#[allow(unused_imports)]
use crate::types::tag::{Tag, TagType};

use std::fs::File;

#[allow(unused_variables)]
pub(crate) fn write_to(data: &mut File, tag: &Tag) -> Result<()> {
	match tag.tag_type() {
		#[cfg(feature = "ape")]
		TagType::Ape => ape::tag::ApeTagRef {
			read_only: false,
			items: ape::tag::tagitems_into_ape(tag.items()),
		}
		.write_to(data),
		#[cfg(feature = "id3v1")]
		TagType::Id3v1 => Into::<v1::tag::Id3v1TagRef<'_>>::into(tag).write_to(data),
		_ => Err(LoftyError::new(ErrorKind::UnsupportedTag)),
	}
}
