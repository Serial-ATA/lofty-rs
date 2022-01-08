use crate::error::{LoftyError, Result};
#[cfg(feature = "id3v2")]
use crate::id3::v2::{
	tag::{tag_frames, Id3v2TagRef},
	Id3v2TagFlags,
};
#[cfg(feature = "aiff_text_chunks")]
use crate::iff::aiff::tag::AiffTextChunksRef;
use crate::types::item::ItemKey;
#[allow(unused_imports)]
use crate::types::tag::{Tag, TagType};

use std::fs::File;

#[allow(unused_variables)]
pub(crate) fn write_to(data: &mut File, tag: &Tag) -> Result<()> {
	match tag.tag_type() {
		#[cfg(feature = "aiff_text_chunks")]
		TagType::AiffText => AiffTextChunksRef::new(
			tag.get_string(&ItemKey::TrackTitle),
			tag.get_string(&ItemKey::TrackArtist),
			tag.get_string(&ItemKey::CopyrightMessage),
			Some(tag.get_texts(&ItemKey::Comment)),
			None,
		)
		.write_to(data),
		#[cfg(feature = "id3v2")]
		TagType::Id3v2 => Id3v2TagRef::new(Id3v2TagFlags::default(), tag_frames(tag)).write_to(data),
		_ => Err(LoftyError::UnsupportedTag),
	}
}
