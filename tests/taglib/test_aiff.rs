use lofty::{Accessor, AudioFile, FileType, TaggedFileExt};

use std::io::Seek;

use crate::util::get_filetype;
use crate::{assert_delta, temp_file};

#[test]
#[ignore]
fn test_aiff_properties() {
	let file = lofty::read_from_path("tests/taglib/data/empty.aiff").unwrap();

	assert_eq!(file.file_type(), FileType::AIFF);

	let properties = file.properties();
	assert_eq!(properties.duration().as_secs(), 0);
	assert_delta!(properties.duration().as_millis(), 67, 1);
	assert_delta!(properties.audio_bitrate().unwrap(), 706, 1);
	assert_eq!(properties.sample_rate(), Some(44100));
	assert_eq!(properties.channels(), Some(1));
	assert_eq!(properties.bit_depth(), Some(16));
	// TODO: get those options in lofty
	// CPPUNIT_ASSERT_EQUAL(2941U, f.audioProperties()->sampleFrames());
	// CPPUNIT_ASSERT_EQUAL(false, f.audioProperties()->isAiffC());
}

#[test]
#[ignore]
fn test_aifc_properties() {
	let file = lofty::read_from_path("tests/taglib/data/alaw.aifc").unwrap();

	assert_eq!(file.file_type(), FileType::AIFF);

	let properties = file.properties();
	assert_eq!(properties.duration().as_secs(), 0);
	assert_delta!(properties.duration().as_millis(), 37, 1);
	assert_eq!(properties.audio_bitrate(), Some(355));
	assert_eq!(properties.sample_rate(), Some(44100));
	assert_eq!(properties.channels(), Some(1));
	assert_eq!(properties.bit_depth(), Some(16));
	// TODO: get those options in lofty
	// CPPUNIT_ASSERT_EQUAL(1622U, f.audioProperties()->sampleFrames());
	// CPPUNIT_ASSERT_EQUAL(true, f.audioProperties()->isAiffC());
	// CPPUNIT_ASSERT_EQUAL(ByteVector("ALAW"), f.audioProperties()->compressionType());
	// CPPUNIT_ASSERT_EQUAL(String("SGI CCITT G.711 A-law"), f.audioProperties()->compressionName());
}

#[test]
#[ignore]
fn test_save_id3v2() {
	let mut file = temp_file!("tests/taglib/data/empty.aiff");

	{
		let mut tfile = lofty::read_from(&mut file).unwrap();

		assert_eq!(tfile.file_type(), FileType::AIFF);

		assert!(tfile.tag(lofty::TagType::ID3v2).is_none());

		let mut tag = lofty::Tag::new(lofty::TagType::ID3v2);
		tag.set_title("TitleXXX".to_string());
		tfile.insert_tag(tag);
		file.rewind().unwrap();
		tfile.save_to(&mut file).unwrap();
		assert!(tfile.contains_tag_type(lofty::TagType::ID3v2));
	}

	file.rewind().unwrap();

	{
		let mut tfile = lofty::read_from(&mut file).unwrap();

		assert_eq!(tfile.file_type(), FileType::AIFF);

		let mut tag = tfile.tag(lofty::TagType::ID3v2).unwrap().to_owned();
		assert_eq!(tag.title().as_deref(), Some("TitleXXX"));
		tag.set_title("".to_string());
		tfile.insert_tag(tag);
		file.rewind().unwrap();
		tfile.save_to(&mut file).unwrap();
		assert!(!tfile.contains_tag_type(lofty::TagType::ID3v2));
	}

	file.rewind().unwrap();

	{
		let tfile = lofty::read_from(&mut file).unwrap();

		assert_eq!(tfile.file_type(), FileType::AIFF);

		assert!(!tfile.contains_tag_type(lofty::TagType::ID3v2));
	}
}

// TODO: testSaveID3v23
// TODO: testDuplicateID3v2

#[test]
#[ignore]
fn test_fuzzed_file1() {
	assert_eq!(
		get_filetype("tests/taglib/data/segfault.aif"),
		FileType::AIFF
	);
}

// the file doesn't even have a valid signature
// #[test]
// #[ignore]
// fn test_fuzzed_file2() {
// let mut file = File::open("tests/taglib/data/excessive_alloc.aif").unwrap();
//
// let mut buf = [0; 12];
// file.read_exact(&mut buf).unwrap();
//
// assert_eq!(FileType::from_buffer(&buf).unwrap(), FileType::AIFF);
// }
