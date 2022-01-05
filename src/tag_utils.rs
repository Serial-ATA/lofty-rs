#[cfg(feature = "ape")]
use crate::ape::tag::ape_tag::ApeTagRef;
use crate::error::{LoftyError, Result};
#[cfg(feature = "id3v1")]
use crate::id3::v1::tag::Id3v1TagRef;
#[cfg(feature = "id3v2")]
use crate::id3::v2::tag::Id3v2TagRef;
#[cfg(feature = "aiff_text_chunks")]
use crate::iff::aiff::tag::AiffTextChunksRef;
#[cfg(feature = "riff_info_list")]
use crate::iff::wav::tag::RiffInfoListRef;
#[cfg(feature = "mp4_ilst")]
use crate::mp4::ilst::IlstRef;
#[cfg(feature = "vorbis_comments")]
use crate::ogg::{
	constants::{OPUSTAGS, VORBIS_COMMENT_HEAD},
	tag::VorbisCommentsRef,
};
use crate::types::file::FileType;
use crate::types::item::ItemKey;
use crate::types::tag::{Tag, TagType};

use std::fs::File;
use std::io::Write;

#[allow(unreachable_patterns)]
pub(crate) fn write_tag(tag: &Tag, file: &mut File, file_type: FileType) -> Result<()> {
	match file_type {
		FileType::AIFF => crate::iff::aiff::write::write_to(file, tag),
		FileType::APE => crate::ape::write::write_to(file, tag),
		#[cfg(feature = "vorbis_comments")]
		FileType::FLAC => {
			crate::ogg::flac::write::write_to(file, &mut Into::<VorbisCommentsRef>::into(tag))
		},
		FileType::MP3 => crate::mp3::write::write_to(file, tag),
		#[cfg(feature = "mp4_ilst")]
		FileType::MP4 => crate::mp4::ilst::write::write_to(file, &mut Into::<IlstRef>::into(tag)),
		#[cfg(feature = "vorbis_comments")]
		FileType::Opus => crate::ogg::write::write_to(file, tag, OPUSTAGS),
		#[cfg(feature = "vorbis_comments")]
		FileType::Vorbis => crate::ogg::write::write_to(file, tag, VORBIS_COMMENT_HEAD),
		FileType::WAV => crate::iff::wav::write::write_to(file, tag),
		_ => Err(LoftyError::UnsupportedTag),
	}
}

#[allow(unreachable_patterns)]
pub(crate) fn dump_tag<W: Write>(tag: &Tag, writer: &mut W) -> Result<()> {
	match tag.tag_type() {
		#[cfg(feature = "ape")]
		TagType::Ape => Into::<ApeTagRef>::into(tag).dump_to(writer),
		#[cfg(feature = "id3v1")]
		TagType::Id3v1 => Into::<Id3v1TagRef>::into(tag).dump_to(writer),
		#[cfg(feature = "id3v2")]
		TagType::Id3v2 => Into::<Id3v2TagRef>::into(tag).dump_to(writer),
		#[cfg(feature = "mp4_ilst")]
		TagType::Mp4Ilst => Into::<IlstRef>::into(tag).dump_to(writer),
		#[cfg(feature = "vorbis_comments")]
		TagType::VorbisComments => Into::<VorbisCommentsRef>::into(tag).dump_to(writer),
		#[cfg(feature = "riff_info_list")]
		TagType::RiffInfo => Into::<RiffInfoListRef>::into(tag).dump_to(writer),
		#[cfg(feature = "aiff_text_chunks")]
		TagType::AiffText => AiffTextChunksRef::new(
			tag.get_string(&ItemKey::TrackTitle),
			tag.get_string(&ItemKey::TrackArtist),
			tag.get_string(&ItemKey::CopyrightMessage),
			Some(tag.get_texts(&ItemKey::Comment)),
			None,
		)
		.dump_to(writer),
		_ => Ok(()),
	}
}

macro_rules! tag_methods {
	(
		$(
			$(#[$attr:meta])?;
			$name:ident,
			$ty:ty
		);*
	) => {
		paste::paste! {
			$(
				$(#[$attr])?
				#[doc = "Gets the [`" $ty "`] if it exists"]
				pub fn $name(&self) -> Option<&$ty> {
					self.$name.as_ref()
				}

				$(#[$attr])?
				#[doc = "Gets a mutable reference to the [`" $ty "`] if it exists"]
				pub fn [<$name _mut>](&mut self) -> Option<&mut $ty> {
					self.$name.as_mut()
				}

				$(#[$attr])?
				#[doc = "Removes the [`" $ty "`]"]
				pub fn [<remove_ $name>](&mut self) {
					self.$name = None
				}
			)*
		}
	}
}

pub(crate) use tag_methods;

#[cfg(test)]
// Used for tag conversion tests
pub(crate) mod test_utils {
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
