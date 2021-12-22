pub(crate) mod ape;
pub(crate) mod id3;
pub(crate) mod iff;
pub(crate) mod mp3;
pub(crate) mod mp4;
pub(crate) mod ogg;

use crate::error::{LoftyError, Result};
use crate::types::file::FileType;
use crate::types::tag::Tag;
#[cfg(feature = "mp4_ilst")]
use mp4::ilst::IlstRef;
#[cfg(feature = "vorbis_comments")]
use ogg::{
	constants::{OPUSTAGS, VORBIS_COMMENT_HEAD},
	tag::VorbisCommentsRef,
};

use std::fs::File;

#[allow(unreachable_patterns)]
pub(crate) fn write_tag(tag: &Tag, file: &mut File, file_type: FileType) -> Result<()> {
	match file_type {
		FileType::AIFF => iff::aiff::write::write_to(file, tag),
		FileType::APE => ape::write::write_to(file, tag),
		#[cfg(feature = "vorbis_comments")]
		FileType::FLAC => ogg::flac::write::write_to(file, &mut Into::<VorbisCommentsRef>::into(tag)),
		FileType::MP3 => mp3::write::write_to(file, tag),
		#[cfg(feature = "mp4_ilst")]
		FileType::MP4 => mp4::ilst::write::write_to(file, &mut Into::<IlstRef>::into(tag)),
		#[cfg(feature = "vorbis_comments")]
		FileType::Opus => ogg::write::write_to(file, tag, OPUSTAGS),
		#[cfg(feature = "vorbis_comments")]
		FileType::Vorbis => ogg::write::write_to(file, tag, VORBIS_COMMENT_HEAD),
		FileType::WAV => iff::wav::write::write_to(file, tag),
		_ => Err(LoftyError::UnsupportedTag),
	}
}

macro_rules! tag_methods {
	($(
	$(#[$attr:meta])?;
	$display_name:tt,
	$name:ident,
	$ty:ty);*
	) => {
		paste::paste! {
			$(
				$(#[$attr])?
				#[doc = "Gets the " $display_name "tag if it exists"]
				pub fn $name(&self) -> Option<&$ty> {
					self.$name.as_ref()
				}

				$(#[$attr])?
				#[doc = "Sets the " $display_name]
				pub fn [<set_ $name>](&mut self, tag: $ty) {
					self.$name = Some(tag)
				}

				$(#[$attr])?
				#[doc = "Removes the " $display_name]
				pub fn [<remove_ $name>](&mut self) {
					self.$name = None
				}
			)*
		}
	}
}

pub(in crate::logic) use tag_methods;

#[cfg(test)]
// Used for tag conversion tests
mod test_utils {
	use crate::{ItemKey, Tag, TagType};

	pub(crate) fn create_tag(tag_type: TagType) -> Tag {
		let mut tag = Tag::new(tag_type);

		tag.insert_text(ItemKey::TrackTitle, String::from("Foo title"));
		tag.insert_text(ItemKey::TrackArtist, String::from("Bar artist"));
		tag.insert_text(ItemKey::AlbumTitle, String::from("Baz album"));
		tag.insert_text(ItemKey::Comment, String::from("Qux comment"));
		tag.insert_text(ItemKey::TrackNumber, String::from("1"));
		tag.insert_text(ItemKey::Genre, String::from("Classical"));

		tag
	}

	pub(crate) fn verify_tag(tag: &Tag, track_number: bool, genre: bool) {
		assert_eq!(tag.get_string(&ItemKey::TrackTitle), Some("Foo title"));
		assert_eq!(tag.get_string(&ItemKey::TrackArtist), Some("Bar artist"));
		assert_eq!(tag.get_string(&ItemKey::AlbumTitle), Some("Baz album"));
		assert_eq!(tag.get_string(&ItemKey::Comment), Some("Qux comment"));

		if track_number {
			assert_eq!(tag.get_string(&ItemKey::TrackNumber), Some("1"));
		}

		if genre {
			assert_eq!(tag.get_string(&ItemKey::Genre), Some("Classical"));
		}
	}
}
