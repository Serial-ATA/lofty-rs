use crate::ape::ApeTag;
use crate::config::WriteOptions;
use crate::error::{FileEncodingError, TagEncodingError};
use crate::file::FileType;
use crate::id3::lyrics3::v1::Lyrics3Tag;
use crate::id3::lyrics3::v2::Lyrics3v2Tag;
use crate::id3::v1::Id3v1Tag;
use crate::id3::v2::Id3v2Tag;
use crate::iff::aiff::AiffTextChunks;
use crate::iff::wav::RiffInfoList;
use crate::io::VerifiedFile;
use crate::mp4::Ilst;
use crate::ogg::tag::VorbisComments;
use crate::tag::{Tag, TagRef, TagRefExt, TagType};
use crate::util::io::FileLike;

use std::io::Write;

pub(crate) fn write_tag<F>(
	tag: &Tag,
	file: VerifiedFile<'_, F>,
	write_options: WriteOptions,
) -> Result<(), FileEncodingError>
where
	F: FileLike,
{
	if matches!(file.format(), FileType::Custom(_)) {
		unreachable!("custom file types don't support writing");
	}

	match tag.tag_type() {
		TagType::Ape => ApeTag::as_tag_ref(tag).write_to(file, write_options),
		TagType::Id3v1 => Id3v1Tag::as_tag_ref(tag).write_to(file, write_options),
		TagType::Id3v2 => Id3v2Tag::as_tag_ref(tag).write_to(file, write_options),
		TagType::Lyrics3 => Lyrics3Tag::as_tag_ref(tag).write_to(file, write_options),
		TagType::Lyrics3v2 => Lyrics3v2Tag::as_tag_ref(tag).write_to(file, write_options),
		TagType::Mp4Ilst => Ilst::as_tag_ref(tag).write_to(file, write_options),
		TagType::VorbisComments => VorbisComments::as_tag_ref(tag).write_to(file, write_options),
		TagType::RiffInfo => RiffInfoList::as_tag_ref(tag).write_to(file, write_options),
		TagType::AiffText => AiffTextChunks::as_tag_ref(tag).write_to(file, write_options),
	}
}

pub(crate) fn dump_tag<W: Write>(
	tag: &Tag,
	writer: &mut W,
	write_options: WriteOptions,
) -> Result<(), TagEncodingError> {
	match tag.tag_type() {
		TagType::Ape => ApeTag::as_tag_ref(tag).dump_to(writer, write_options),
		TagType::Id3v1 => Id3v1Tag::as_tag_ref(tag).dump_to(writer, write_options),
		TagType::Id3v2 => Id3v2Tag::as_tag_ref(tag).dump_to(writer, write_options),
		TagType::Lyrics3 => Lyrics3Tag::as_tag_ref(tag).dump_to(writer, write_options),
		TagType::Lyrics3v2 => Lyrics3v2Tag::as_tag_ref(tag).dump_to(writer, write_options),
		TagType::Mp4Ilst => Ilst::as_tag_ref(tag).dump_to(writer, write_options),
		TagType::VorbisComments => VorbisComments::as_tag_ref(tag).dump_to(writer, write_options),
		TagType::RiffInfo => RiffInfoList::as_tag_ref(tag).dump_to(writer, write_options),
		TagType::AiffText => AiffTextChunks::as_tag_ref(tag).dump_to(writer, write_options),
	}
}

#[cfg(test)]
// Used for tag conversion tests
pub(crate) mod test_utils {
	use crate::tag::{ItemKey, Tag, TagType};
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
		assert_eq!(tag.get_string(ItemKey::TrackTitle), Some("Foo title"));
		assert_eq!(tag.get_string(ItemKey::TrackArtist), Some("Bar artist"));
		assert_eq!(tag.get_string(ItemKey::AlbumTitle), Some("Baz album"));
		assert_eq!(tag.get_string(ItemKey::Comment), Some("Qux comment"));

		if track_number {
			assert_eq!(tag.get_string(ItemKey::TrackNumber), Some("1"));
		}

		if genre {
			assert_eq!(tag.get_string(ItemKey::Genre), Some("Classical"));
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
