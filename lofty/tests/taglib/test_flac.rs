use crate::temp_file;
use crate::util::get_file;

use std::io::{Read, Seek, SeekFrom};

use lofty::config::{ParseOptions, WriteOptions};
use lofty::file::AudioFile;
use lofty::flac::FlacFile;
use lofty::id3::v2::Id3v2Tag;
use lofty::ogg::{OggPictureStorage, VorbisComments};
use lofty::picture::{MimeType, Picture, PictureInformation, PictureType};
use lofty::tag::{Accessor, TagExt};

#[test_log::test]
fn test_signature() {
	let f = get_file::<FlacFile>("tests/taglib/data/no-tags.flac");
	assert_eq!(
		format!("{:x}", f.properties().signature()),
		"a1b141f766e9849ac3db1030a20a3c77"
	);
}

#[test_log::test]
#[ignore = "Marker test, Lofty does not replicate TagLib's behavior here"]
fn test_multiple_comment_blocks() {
	// TagLib will use the *first* tag in the stream, while we use the latest.
}

#[test_log::test]
fn test_read_picture() {
	let mut file = temp_file!("tests/taglib/data/silence-44-s.flac");
	let f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
	let lst = f.pictures();
	assert_eq!(lst.len(), 1);

	let (pic, info) = &lst[0];
	assert_eq!(pic.pic_type(), PictureType::CoverFront);
	assert_eq!(info.width, 1);
	assert_eq!(info.height, 1);
	assert_eq!(info.color_depth, 24);
	assert_eq!(info.num_colors, 0);
	assert_eq!(pic.mime_type(), Some(&MimeType::Png));
	assert_eq!(pic.description(), Some("A pixel."));
	assert_eq!(pic.data().len(), 150);
}

#[test_log::test]
fn test_add_picture() {
	let mut file = temp_file!("tests/taglib/data/silence-44-s.flac");
	{
		let mut f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		let lst = f.pictures();
		assert_eq!(lst.len(), 1);

		let new_pic = Picture::unchecked(Vec::from("JPEG data"))
			.pic_type(PictureType::CoverBack)
			.mime_type(MimeType::Jpeg)
			.description("new image")
			.build();
		let new_pic_info = PictureInformation {
			width: 5,
			height: 6,
			color_depth: 16,
			num_colors: 7,
		};

		f.insert_picture(new_pic, Some(new_pic_info)).unwrap();
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		let lst = f.pictures();
		assert_eq!(lst.len(), 2);

		let (pic, info) = &lst[0];
		assert_eq!(pic.pic_type(), PictureType::CoverFront);
		assert_eq!(info.width, 1);
		assert_eq!(info.height, 1);
		assert_eq!(info.color_depth, 24);
		assert_eq!(info.num_colors, 0);
		assert_eq!(pic.mime_type(), Some(&MimeType::Png));
		assert_eq!(pic.description(), Some("A pixel."));
		assert_eq!(pic.data().len(), 150);

		let (pic, info) = &lst[1];
		assert_eq!(pic.pic_type(), PictureType::CoverBack);
		assert_eq!(info.width, 5);
		assert_eq!(info.height, 6);
		assert_eq!(info.color_depth, 16);
		assert_eq!(info.num_colors, 7);
		assert_eq!(pic.mime_type(), Some(&MimeType::Jpeg));
		assert_eq!(pic.description(), Some("new image"));
		assert_eq!(pic.data(), b"JPEG data");
	}
}

#[test_log::test]
fn test_replace_picture() {
	let mut file = temp_file!("tests/taglib/data/silence-44-s.flac");
	{
		let mut f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		let lst = f.pictures();
		assert_eq!(lst.len(), 1);

		let new_pic = Picture::unchecked(Vec::from("JPEG data"))
			.pic_type(PictureType::CoverBack)
			.mime_type(MimeType::Jpeg)
			.description("new image")
			.build();
		let new_pic_info = PictureInformation {
			width: 5,
			height: 6,
			color_depth: 16,
			num_colors: 7,
		};

		f.remove_pictures();
		f.insert_picture(new_pic, Some(new_pic_info)).unwrap();
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		let lst = f.pictures();
		assert_eq!(lst.len(), 1);

		let (pic, info) = &lst[0];
		assert_eq!(pic.pic_type(), PictureType::CoverBack);
		assert_eq!(info.width, 5);
		assert_eq!(info.height, 6);
		assert_eq!(info.color_depth, 16);
		assert_eq!(info.num_colors, 7);
		assert_eq!(pic.mime_type(), Some(&MimeType::Jpeg));
		assert_eq!(pic.description(), Some("new image"));
		assert_eq!(pic.data(), b"JPEG data");
	}
}

#[test_log::test]
fn test_remove_all_pictures() {
	let mut file = temp_file!("tests/taglib/data/silence-44-s.flac");
	{
		let mut f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		let lst = f.pictures();
		assert_eq!(lst.len(), 1);

		f.remove_pictures();
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		let lst = f.pictures();
		assert_eq!(lst.len(), 0);
	}
}

#[test_log::test]
fn test_repeated_save_1() {
	let mut file = temp_file!("tests/taglib/data/silence-44-s.flac");
	{
		let mut f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		assert_eq!(
			f.vorbis_comments().unwrap().title().as_deref(),
			Some("Silence")
		);
		f.vorbis_comments_mut()
			.unwrap()
			.set_title(String::from("NEW TITLE"));
		f.save_to(&mut file, WriteOptions::default()).unwrap();

		file.rewind().unwrap();
		assert_eq!(
			f.vorbis_comments().unwrap().title().as_deref(),
			Some("NEW TITLE")
		);
		f.vorbis_comments_mut()
			.unwrap()
			.set_title(String::from("NEW TITLE 2"));
		f.save_to(&mut file, WriteOptions::default()).unwrap();

		assert_eq!(
			f.vorbis_comments().unwrap().title().as_deref(),
			Some("NEW TITLE 2")
		);
	}
	file.rewind().unwrap();
	{
		let f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();
		assert_eq!(
			f.vorbis_comments().unwrap().title().as_deref(),
			Some("NEW TITLE 2")
		);
	}
}

#[test_log::test]
#[ignore = "Marker test, this test relies on saving an ID3v2 tag in a FLAC file, something Lofty \
            does not and will not support."]
fn test_repeated_save_2() {}

// TODO: We don't make use of padding blocks yet
#[test_log::test]
#[ignore = "FLAC padding blocks aren't used yet"]
fn test_repeated_save_3() {
	let mut file = temp_file!("tests/taglib/data/no-tags.flac");

	let mut f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
	file.rewind().unwrap();

	let mut tag = VorbisComments::default();
	tag.set_title(String::from_utf8(vec![b'X'; 8 * 1024]).unwrap());
	f.set_vorbis_comments(tag);

	f.save_to(&mut file, WriteOptions::default()).unwrap();
	file.rewind().unwrap();
	assert_eq!(file.metadata().unwrap().len(), 12862);
	f.save_to(&mut file, WriteOptions::default()).unwrap();
	assert_eq!(file.metadata().unwrap().len(), 12862);
}

#[test_log::test]
fn test_save_multiple_values() {
	let mut file = temp_file!("tests/taglib/data/silence-44-s.flac");
	{
		let mut f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		f.vorbis_comments_mut()
			.unwrap()
			.insert(String::from("ARTIST"), String::from("artist 1"));
		f.vorbis_comments_mut()
			.unwrap()
			.push(String::from("ARTIST"), String::from("artist 2"));
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
		let mut m = f.vorbis_comments().unwrap().get_all("ARTIST");
		assert_eq!(m.next(), Some("artist 1"));
		assert_eq!(m.next(), Some("artist 2"));
		assert_eq!(m.next(), None);
	}
}

#[test_log::test]
#[ignore = "Marker test, Lofty does not replicate the dict API"]
fn test_dict() {}

#[test_log::test]
fn test_properties() {
	let mut tag = VorbisComments::default();
	tag.push(String::from("ALBUM"), String::from("Album"));
	tag.push(String::from("ALBUMARTIST"), String::from("Album Artist"));
	tag.push(
		String::from("ALBUMARTISTSORT"),
		String::from("Album Artist Sort"),
	);
	tag.push(String::from("ALBUMSORT"), String::from("Album Sort"));
	tag.push(String::from("ARTIST"), String::from("Artist"));
	tag.push(String::from("ARTISTS"), String::from("Artists"));
	tag.push(String::from("ARTISTSORT"), String::from("Artist Sort"));
	tag.push(String::from("ASIN"), String::from("ASIN"));
	tag.push(String::from("BARCODE"), String::from("Barcode"));
	tag.push(
		String::from("CATALOGNUMBER"),
		String::from("Catalog Number 1"),
	);
	tag.push(
		String::from("CATALOGNUMBER"),
		String::from("Catalog Number 2"),
	);
	tag.push(String::from("COMMENT"), String::from("Comment"));
	tag.push(String::from("DATE"), String::from("2021-01-10"));
	tag.push(String::from("DISCNUMBER"), String::from("3/5"));
	tag.push(String::from("GENRE"), String::from("Genre"));
	tag.push(String::from("ISRC"), String::from("UKAAA0500001"));
	tag.push(String::from("LABEL"), String::from("Label 1"));
	tag.push(String::from("LABEL"), String::from("Label 2"));
	tag.push(String::from("MEDIA"), String::from("Media"));
	tag.push(
		String::from("MUSICBRAINZ_ALBUMARTISTID"),
		String::from("MusicBrainz_AlbumartistID"),
	);
	tag.push(
		String::from("MUSICBRAINZ_ALBUMID"),
		String::from("MusicBrainz_AlbumID"),
	);
	tag.push(
		String::from("MUSICBRAINZ_ARTISTID"),
		String::from("MusicBrainz_ArtistID"),
	);
	tag.push(
		String::from("MUSICBRAINZ_RELEASEGROUPID"),
		String::from("MusicBrainz_ReleasegroupID"),
	);
	tag.push(
		String::from("MUSICBRAINZ_RELEASETRACKID"),
		String::from("MusicBrainz_ReleasetrackID"),
	);
	tag.push(
		String::from("MUSICBRAINZ_TRACKID"),
		String::from("MusicBrainz_TrackID"),
	);
	tag.push(String::from("ORIGINALDATE"), String::from("2021-01-09"));
	tag.push(
		String::from("RELEASECOUNTRY"),
		String::from("Release Country"),
	);
	tag.push(
		String::from("RELEASESTATUS"),
		String::from("Release Status"),
	);
	tag.push(String::from("RELEASETYPE"), String::from("Release Type"));
	tag.push(String::from("SCRIPT"), String::from("Script"));
	tag.push(String::from("TITLE"), String::from("Title"));
	tag.push(String::from("TRACKNUMBER"), String::from("2"));
	tag.push(String::from("TRACKTOTAL"), String::from("4"));

	let mut file = temp_file!("tests/taglib/data/no-tags.flac");
	{
		let mut f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();

		f.set_vorbis_comments(tag.clone());

		file.rewind().unwrap();
		f.vorbis_comments()
			.unwrap()
			.save_to(&mut file, WriteOptions::default())
			.unwrap();
	}
	file.rewind().unwrap();
	{
		// <current>/<total> DISCNUMBER is a special case in Lofty, so disable implicit_conversions
		// to match TagLib
		let f = FlacFile::read_from(&mut file, ParseOptions::new().implicit_conversions(false))
			.unwrap();

		assert_eq!(f.vorbis_comments(), Some(&tag));
	}
}

#[test_log::test]
fn test_invalid() {
	let mut file = temp_file!("tests/taglib/data/silence-44-s.flac");
	let mut f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();

	// NOTE: In TagLib, there's a `setProperties` method. This is equivalent.
	f.vorbis_comments_mut().unwrap().clear();

	f.vorbis_comments_mut()
		.unwrap()
		.push(String::from("H\x00c4\x00d6"), String::from("bla"));
	assert!(f.vorbis_comments().unwrap().is_empty());
}

#[test_log::test]
fn test_audio_properties() {
	let f = get_file::<FlacFile>("tests/taglib/data/sinewave.flac");

	let properties = f.properties();
	assert_eq!(properties.duration().as_secs(), 3);
	assert_eq!(properties.duration().as_millis(), 3550);
	assert_eq!(properties.audio_bitrate(), 145);
	assert_eq!(properties.sample_rate(), 44100);
	assert_eq!(properties.channels(), 2);
	assert_eq!(properties.bit_depth(), 16);
	// TODO
	// CPPUNIT_ASSERT_EQUAL(156556ULL, f.audioProperties()->sampleFrames());
	assert_eq!(
		format!("{:X}", f.properties().signature()),
		"CFE3D9DABADEAB2CBF2CA235274B7F76"
	);
}

#[test_log::test]
fn test_zero_sized_padding_1() {
	let _f = get_file::<FlacFile>("tests/taglib/data/zero-sized-padding.flac");
}

#[test_log::test]
fn test_zero_sized_padding_2() {
	let mut file = temp_file!("tests/taglib/data/silence-44-s.flac");
	{
		let mut f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		f.vorbis_comments_mut()
			.unwrap()
			.set_title(String::from("ABC"));
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let mut f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		f.vorbis_comments_mut()
			.unwrap()
			.set_title(String::from_utf8(vec![b'X'; 3067]).unwrap());
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let _ = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
	}
}

// TODO: We don't make use of padding blocks yet
#[test_log::test]
#[ignore = "FLAC padding blocks aren't used yet"]
fn test_shrink_padding() {
	let mut file = temp_file!("tests/taglib/data/silence-44-s.flac");
	{
		let mut f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		f.vorbis_comments_mut()
			.unwrap()
			.set_title(String::from_utf8(vec![b'X'; 128 * 1024]).unwrap());
		f.save_to(&mut file, WriteOptions::default()).unwrap();
		assert!(file.metadata().unwrap().len() > 128 * 1024);
	}
	file.rewind().unwrap();
	{
		let mut f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		f.vorbis_comments_mut()
			.unwrap()
			.set_title(String::from("0123456789"));
		f.save_to(&mut file, WriteOptions::default()).unwrap();
		assert!(file.metadata().unwrap().len() < 8 * 1024);
	}
}

#[test_log::test]
#[ignore = "Marker test, this test relies on saving an ID3v1 tag in a FLAC file, something Lofty \
            does not and will not support."]
fn test_save_id3v1() {}

#[test_log::test]
#[ignore = "Marker test, this test relies on saving an ID3v2 tag in a FLAC file, something Lofty \
            does not and will not support."]
fn test_update_id3v2() {}

#[test_log::test]
fn test_empty_id3v2() {
	let mut file = temp_file!("tests/taglib/data/no-tags.flac");
	{
		let mut f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		f.set_id3v2(Id3v2Tag::default());
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
		assert!(f.id3v2().is_none());
	}
}

// TODO: TagLib doesn't fully remove Vorbis Comments when stripping. It will preserve the vendor string. Should we do the same?
#[test_log::test]
#[ignore = "Needs to be looked into more"]
fn test_strip_tags() {
	// NOTE: In the TagLib test suite, this also tests ID3v1 and ID3v2. That is not replicated here.

	let mut file = temp_file!("tests/taglib/data/silence-44-s.flac");
	{
		let mut f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		f.vorbis_comments_mut()
			.unwrap()
			.set_title(String::from("XiphComment Title"));
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let mut f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		assert!(f.vorbis_comments().is_some());
		assert_eq!(
			f.vorbis_comments().unwrap().title().as_deref(),
			Some("XiphComment Title")
		);
		f.remove_vorbis_comments();
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		assert!(f.vorbis_comments().is_some());
		assert!(f.vorbis_comments().unwrap().is_empty());
		assert_eq!(
			f.vorbis_comments().unwrap().vendor(),
			"reference libFLAC 1.1.0 20030126"
		);
	}
}

#[test_log::test]
fn test_remove_xiph_field() {
	let mut file = temp_file!("tests/taglib/data/silence-44-s.flac");
	{
		let mut f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		f.vorbis_comments_mut()
			.unwrap()
			.set_title(String::from("XiphComment Title"));
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let mut f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		assert_eq!(
			f.vorbis_comments().unwrap().title().as_deref(),
			Some("XiphComment Title")
		);
		let _ = f.vorbis_comments_mut().unwrap().remove("TITLE");
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		assert!(f.vorbis_comments().unwrap().title().is_none());
	}
}

#[test_log::test]
fn test_empty_seek_table() {
	let mut file = temp_file!("tests/taglib/data/empty-seektable.flac");
	{
		let mut f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		let mut tag = VorbisComments::default();
		tag.set_title(String::from("XiphComment Title"));
		f.set_vorbis_comments(tag);
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();

		// NOTE: TagLib has this at offset 42. This is because we always shift any blocks we write
		//       to be immediately after `STREAMINFO`, whereas TagLib will append them to the end.
		file.seek(SeekFrom::Start(113)).unwrap();
		assert!(f.vorbis_comments().is_some());

		let mut data = [0; 4];
		file.read_exact(&mut data).unwrap();
		assert_eq!(data, [3, 0, 0, 0]);
	}
}

#[test_log::test]
fn test_picture_stored_after_comment() {
	// Blank.png from https://commons.wikimedia.org/wiki/File:Blank.png
	const BLANK_PNG_DATA: &[u8] = &[
		0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
		0x52, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x02, 0x08, 0x06, 0x00, 0x00, 0x00, 0x9D,
		0x74, 0x66, 0x1A, 0x00, 0x00, 0x00, 0x01, 0x73, 0x52, 0x47, 0x42, 0x00, 0xAE, 0xCE, 0x1C,
		0xE9, 0x00, 0x00, 0x00, 0x04, 0x67, 0x41, 0x4D, 0x41, 0x00, 0x00, 0xB1, 0x8F, 0x0B, 0xFC,
		0x61, 0x05, 0x00, 0x00, 0x00, 0x09, 0x70, 0x48, 0x59, 0x73, 0x00, 0x00, 0x0E, 0xC3, 0x00,
		0x00, 0x0E, 0xC3, 0x01, 0xC7, 0x6F, 0xA8, 0x64, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41,
		0x54, 0x18, 0x57, 0x63, 0xC0, 0x01, 0x18, 0x18, 0x00, 0x00, 0x1A, 0x00, 0x01, 0x82, 0x92,
		0x4D, 0x60, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
	];

	let mut file = temp_file!("tests/taglib/data/no-tags.flac");
	{
		let mut f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		assert!(f.id3v2().is_none());
		assert!(f.vorbis_comments().is_none());
		assert!(f.pictures().is_empty());

		let pic = Picture::unchecked(BLANK_PNG_DATA.to_vec())
			.pic_type(PictureType::CoverFront)
			.mime_type(MimeType::Png)
			.description("blank.png")
			.build();
		let pic_information = PictureInformation {
			width: 3,
			height: 2,
			color_depth: 32,
			num_colors: 0,
		};
		f.insert_picture(pic, Some(pic_information)).unwrap();

		let mut tag = VorbisComments::default();
		tag.set_title(String::from("Title"));
		f.set_vorbis_comments(tag);
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		assert!(f.id3v2().is_none());
		assert!(f.vorbis_comments().is_some());

		let pictures = f.pictures();
		assert_eq!(pictures.len(), 1);
		assert_eq!(pictures[0].0.data(), BLANK_PNG_DATA);
		assert_eq!(pictures[0].0.pic_type(), PictureType::CoverFront);
		assert_eq!(pictures[0].0.mime_type(), Some(&MimeType::Png));
		assert_eq!(pictures[0].0.description(), Some("blank.png"));
		assert_eq!(pictures[0].1.width, 3);
		assert_eq!(pictures[0].1.height, 2);
		assert_eq!(pictures[0].1.color_depth, 32);
		assert_eq!(pictures[0].1.num_colors, 0);
		assert_eq!(
			f.vorbis_comments().unwrap().title().as_deref(),
			Some("Title")
		);
	}

	const EXPECTED_HEAD_DATA: &[u8] = &[
		b'f', b'L', b'a', b'C', 0x00, 0x00, 0x00, 0x22, 0x12, 0x00, 0x12, 0x00, 0x00, 0x00, 0x0E,
		0x00, 0x00, 0x10, 0x0A, 0xC4, 0x42, 0xF0, 0x00, 0x02, 0x7A, 0xC0, 0xA1, 0xB1, 0x41, 0xF7,
		0x66, 0xE9, 0x84, 0x9A, 0xC3, 0xDB, 0x10, 0x30, 0xA2, 0x0A, 0x3C, 0x77, 0x04, 0x00, 0x00,
		0x17, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x0B, 0x00, 0x00, 0x00, b'T', b'I',
		b'T', b'L', b'E', b'=', b'T', b'i', b't', b'l', b'e', 0x06, 0x00, 0x00, 0xA9, 0x00, 0x00,
		0x00, 0x03, 0x00, 0x00, 0x00, 0x09, b'i', b'm', b'a', b'g', b'e', b'/', b'p', b'n', b'g',
		0x00, 0x00, 0x00, 0x09, b'b', b'l', b'a', b'n', b'k', b'.', b'p', b'n', b'g', 0x00, 0x00,
		0x00, 0x03, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00,
		0x00, 0x00, 0x77,
	];

	let mut file_data = Vec::new();
	file.read_to_end(&mut file_data).unwrap();

	assert!(file_data.starts_with(EXPECTED_HEAD_DATA));
}
