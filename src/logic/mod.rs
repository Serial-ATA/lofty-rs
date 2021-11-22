pub(crate) mod ape;
pub(crate) mod id3;
pub(crate) mod iff;
pub(crate) mod mp3;
pub(crate) mod mp4;
pub(crate) mod ogg;

use crate::error::Result;
use crate::logic::mp4::ilst::IlstRef;
use crate::logic::ogg::tag::VorbisCommentsRef;
use crate::types::file::FileType;
use crate::types::tag::Tag;
use ogg::constants::{OPUSTAGS, VORBIS_COMMENT_HEAD};

use std::fs::File;

pub(crate) fn write_tag(tag: &Tag, file: &mut File, file_type: FileType) -> Result<()> {
	match file_type {
		FileType::AIFF => iff::aiff::write::write_to(file, tag),
		FileType::APE => ape::write::write_to(file, tag),
		FileType::FLAC => {
			ogg::flac::write::write_to(file, &mut Into::<VorbisCommentsRef>::into(tag))
		},
		FileType::MP3 => mp3::write::write_to(file, tag),
		FileType::MP4 => mp4::ilst::write::write_to(file, &mut Into::<IlstRef>::into(tag)),
		FileType::Opus => ogg::write::write_to(file, tag, OPUSTAGS),
		FileType::Vorbis => ogg::write::write_to(file, tag, VORBIS_COMMENT_HEAD),
		FileType::WAV => iff::wav::write::write_to(file, tag),
	}
}

macro_rules! tag_methods {
	($impl_for:ident => $($display_name:tt, $name:ident, $ty:ty);*) => {
		impl $impl_for {
			paste::paste! {
				$(
					#[doc = "Gets the " $display_name "tag if it exists"]
					pub fn $name(&self) -> Option<&$ty> {
						self.$name.as_ref()
					}

					#[doc = "Sets the " $display_name]
					pub fn [<set_ $name>](&mut self, tag: $ty) {
						self.$name = Some(tag)
					}
				)*
			}
		}
	}
}

pub(in crate::logic) use tag_methods;
