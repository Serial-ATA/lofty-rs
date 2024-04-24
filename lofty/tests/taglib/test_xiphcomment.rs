use crate::temp_file;

use std::io::Seek;

use lofty::config::{ParseOptions, WriteOptions};
use lofty::file::AudioFile;
use lofty::ogg::{OggPictureStorage, VorbisComments, VorbisFile};
use lofty::picture::{MimeType, Picture, PictureInformation, PictureType};
use lofty::tag::{Accessor, TagExt};

#[test]
fn test_year() {
	let mut cmt = VorbisComments::default();
	assert_eq!(cmt.year(), None);
	cmt.push(String::from("YEAR"), String::from("2009"));
	assert_eq!(cmt.year(), Some(2009));

	// NOTE: Lofty will *always* prioritize "YEAR" over "DATE". TagLib doesn't have the same ideas,
	//       so we have to remove "YEAR".
	let _ = cmt.remove("YEAR");

	cmt.push(String::from("DATE"), String::from("2008"));
	assert_eq!(cmt.year(), Some(2008));
}

#[test]
fn test_set_year() {
	let mut cmt = VorbisComments::default();
	cmt.push(String::from("YEAR"), String::from("2009"));
	cmt.push(String::from("DATE"), String::from("2008"));
	cmt.set_year(1995);
	assert!(cmt.get("YEAR").is_none());
	assert_eq!(cmt.get("DATE"), Some("1995"));
}

#[test]
fn test_track() {
	let mut cmt = VorbisComments::default();
	assert_eq!(cmt.track(), None);
	cmt.push(String::from("TRACKNUM"), String::from("7"));
	assert_eq!(cmt.track(), Some(7));
	cmt.push(String::from("TRACKNUMBER"), String::from("8"));
	assert_eq!(cmt.track(), Some(8));
}

#[test]
fn test_set_track() {
	let mut cmt = VorbisComments::default();
	cmt.push(String::from("TRACKNUM"), String::from("7"));
	cmt.push(String::from("TRACKNUMBER"), String::from("8"));
	cmt.set_track(3);
	assert!(cmt.get("TRACKNUM").is_none());
	assert_eq!(cmt.get("TRACKNUMBER"), Some("3"));
}

#[test]
#[ignore]
fn test_invalid_keys1() {
	// Marker test, Lofty does not replicate the properties API
}

#[test]
fn test_invalid_keys2() {
	let mut cmt = VorbisComments::default();
	cmt.push(String::new(), String::new());
	cmt.push(String::from("A=B"), String::new());
	cmt.push(String::from("A~B"), String::new());
	cmt.push(String::from("A\x7F"), String::new());
	cmt.push(String::from("A\u{3456}"), String::new());

	assert!(cmt.is_empty());
}

#[test]
fn test_clear_comment() {
	let mut file = temp_file!("tests/taglib/data/empty.ogg");

	{
		let mut f = VorbisFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		f.vorbis_comments_mut()
			.push(String::from("COMMENT"), String::from("Comment1"));
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let mut f = VorbisFile::read_from(&mut file, ParseOptions::new()).unwrap();
		f.vorbis_comments_mut().remove_comment();
		assert_eq!(f.vorbis_comments().comment(), None);
	}
}

#[test]
#[ignore]
fn test_remove_fields() {
	// Market test, TagLib has some incredibly strange behavior in this test.
	//
	// When adding a field of the same key, TagLib will append each value to the same value.
	// Meaning:
	//
	// tag.insert("title", "Title1", false);
	// tag.insert("title, "Title2", false);
	// assert_eq!(tag.title(), Some("Title1 Title2");
	//
	// Lofty will never behave in this way.
}

#[test]
fn test_picture() {
	let mut file = temp_file!("tests/taglib/data/empty.ogg");

	{
		let mut f = VorbisFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		let picture = Picture::new_unchecked(
			PictureType::CoverBack,
			Some(MimeType::Jpeg),
			Some(String::from("new image")),
			b"JPEG data".to_vec(),
		);
		let info = PictureInformation {
			width: 5,
			height: 6,
			color_depth: 16,
			num_colors: 7,
		};

		f.vorbis_comments_mut()
			.insert_picture(picture, Some(info))
			.unwrap();
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = VorbisFile::read_from(&mut file, ParseOptions::new()).unwrap();
		let pictures = f.vorbis_comments().pictures();
		assert_eq!(pictures.len(), 1);
		assert_eq!(pictures[0].1.width, 5);
		assert_eq!(pictures[0].1.height, 6);
		assert_eq!(pictures[0].1.color_depth, 16);
		assert_eq!(pictures[0].1.num_colors, 7);
		assert_eq!(pictures[0].0.mime_type(), Some(&MimeType::Jpeg));
		assert_eq!(pictures[0].0.description(), Some("new image"));
		assert_eq!(pictures[0].0.data(), b"JPEG data");
	}
}

#[test]
fn test_lowercase_fields() {
	let mut file = temp_file!("tests/taglib/data/lowercase-fields.ogg");

	{
		let f = VorbisFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		assert_eq!(f.vorbis_comments().title().as_deref(), Some("TEST TITLE"));
		assert_eq!(f.vorbis_comments().artist().as_deref(), Some("TEST ARTIST"));
		assert_eq!(f.vorbis_comments().pictures().len(), 1);
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = VorbisFile::read_from(&mut file, ParseOptions::new()).unwrap();
		assert!(!f.vorbis_comments().pictures().is_empty());
	}
}
