use crate::error::{ErrorKind, LoftyError, Result};
#[cfg(feature = "id3v2")]
use crate::id3::v2;
#[allow(unused_imports)]
use crate::tag::{Tag, TagType};

use std::fs::File;

#[allow(unused_variables)]
pub(crate) fn write_to(data: &mut File, tag: &Tag) -> Result<()> {
	match tag.tag_type() {
		#[cfg(feature = "aiff_text_chunks")]
		TagType::AIFFText => {
			use crate::tag::item::ItemKey;

			super::tag::AiffTextChunksRef {
				name: tag.get_string(&ItemKey::TrackTitle),
				author: tag.get_string(&ItemKey::TrackArtist),
				copyright: tag.get_string(&ItemKey::CopyrightMessage),
				annotations: Some(tag.get_strings(&ItemKey::Comment)),
				comments: None,
			}
		}
		.write_to(data),
		#[cfg(feature = "id3v2")]
		TagType::ID3v2 => v2::tag::Id3v2TagRef {
			flags: v2::ID3v2TagFlags::default(),
			frames: v2::tag::tag_frames(tag),
		}
		.write_to(data),
		_ => Err(LoftyError::new(ErrorKind::UnsupportedTag)),
	}
}
