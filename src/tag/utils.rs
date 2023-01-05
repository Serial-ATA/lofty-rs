use crate::error::Result;
use crate::file::FileType;
use crate::macros::err;
use crate::tag::{Tag, TagType};
use crate::{aac, ape, flac, iff, mpeg, wavpack};

#[cfg(feature = "id3v1")]
use crate::id3::v1::tag::Id3v1TagRef;
#[cfg(feature = "id3v2")]
use crate::id3::v2::{self, tag::Id3v2TagRef, ID3v2TagFlags};
use crate::mp4::Ilst;
use crate::ogg::tag::{create_vorbis_comments_ref, VorbisCommentsRef};
#[cfg(feature = "ape")]
use ape::tag::ApeTagRef;
#[cfg(feature = "aiff_text_chunks")]
use iff::aiff::tag::AiffTextChunksRef;
#[cfg(feature = "riff_info_list")]
use iff::wav::tag::RIFFInfoListRef;

use std::fs::File;
use std::io::Write;

#[allow(unreachable_patterns)]
pub(crate) fn write_tag(tag: &Tag, file: &mut File, file_type: FileType) -> Result<()> {
	match file_type {
		FileType::AAC => aac::write::write_to(file, tag),
		FileType::AIFF => iff::aiff::write::write_to(file, tag),
		FileType::APE => ape::write::write_to(file, tag),
		FileType::FLAC => flac::write::write_to(file, tag),
		FileType::Opus | FileType::Speex | FileType::Vorbis => {
			crate::ogg::write::write_to(file, tag, file_type)
		},
		FileType::MPEG => mpeg::write::write_to(file, tag),
		FileType::MP4 => {
			crate::mp4::ilst::write::write_to(file, &mut Into::<Ilst>::into(tag.clone()).as_ref())
		},
		FileType::WAV => iff::wav::write::write_to(file, tag),
		FileType::WavPack => wavpack::write::write_to(file, tag),
		_ => err!(UnsupportedTag),
	}
}

#[allow(unreachable_patterns)]
pub(crate) fn dump_tag<W: Write>(tag: &Tag, writer: &mut W) -> Result<()> {
	match tag.tag_type() {
		#[cfg(feature = "ape")]
		TagType::APE => ApeTagRef {
			read_only: false,
			items: ape::tag::tagitems_into_ape(tag.items()),
		}
		.dump_to(writer),
		#[cfg(feature = "id3v1")]
		TagType::ID3v1 => Into::<Id3v1TagRef<'_>>::into(tag).dump_to(writer),
		#[cfg(feature = "id3v2")]
		TagType::ID3v2 => Id3v2TagRef {
			flags: ID3v2TagFlags::default(),
			frames: v2::tag::tag_frames(tag),
		}
		.dump_to(writer),
		TagType::MP4ilst => Into::<Ilst>::into(tag.clone()).as_ref().dump_to(writer),
		TagType::VorbisComments => {
			let (vendor, items, pictures) = create_vorbis_comments_ref(tag);

			VorbisCommentsRef {
				vendor,
				items,
				pictures,
			}
			.dump_to(writer)
		},
		#[cfg(feature = "riff_info_list")]
		TagType::RIFFInfo => RIFFInfoListRef {
			items: iff::wav::tag::tagitems_into_riff(tag.items()),
		}
		.dump_to(writer),
		#[cfg(feature = "aiff_text_chunks")]
		TagType::AIFFText => {
			use crate::tag::item::ItemKey;

			AiffTextChunksRef {
				name: tag.get_string(&ItemKey::TrackTitle),
				author: tag.get_string(&ItemKey::TrackArtist),
				copyright: tag.get_string(&ItemKey::CopyrightMessage),
				annotations: Some(tag.get_strings(&ItemKey::Comment)),
				comments: None,
			}
		}
		.dump_to(writer),
		_ => Ok(()),
	}
}

#[cfg(test)]
// Used for tag conversion tests
pub(crate) mod test_utils {
	use crate::{ItemKey, Tag, TagType};
	use std::fs::File;
	use std::io::Read;

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

	pub(crate) fn read_path(path: &str) -> Vec<u8> {
		read_file(&mut File::open(path).unwrap())
	}

	pub(crate) fn read_file(file: &mut File) -> Vec<u8> {
		let mut tag = Vec::new();

		file.read_to_end(&mut tag).unwrap();

		tag
	}
}
