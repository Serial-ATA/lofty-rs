use lofty::{Accessor, AudioFile, FileType, ParseOptions, TaggedFileExt};

use lofty::iff::aiff::AiffFile;
use std::io::{Read, Seek};

use crate::util::get_filetype;
use crate::{assert_delta, temp_file};

#[test]
#[ignore]
fn test_aiff_properties() {
	let file = lofty::read_from_path("tests/taglib/data/empty.aiff").unwrap();

	assert_eq!(file.file_type(), FileType::Aiff);

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

	assert_eq!(file.file_type(), FileType::Aiff);

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

		assert_eq!(tfile.file_type(), FileType::Aiff);

		assert!(tfile.tag(lofty::TagType::Id3v2).is_none());

		let mut tag = lofty::Tag::new(lofty::TagType::Id3v2);
		tag.set_title("TitleXXX".to_string());
		tfile.insert_tag(tag);
		file.rewind().unwrap();
		tfile.save_to(&mut file).unwrap();
		assert!(tfile.contains_tag_type(lofty::TagType::Id3v2));
	}

	file.rewind().unwrap();

	{
		let mut tfile = lofty::read_from(&mut file).unwrap();

		assert_eq!(tfile.file_type(), FileType::Aiff);

		let mut tag = tfile.tag(lofty::TagType::Id3v2).unwrap().to_owned();
		assert_eq!(tag.title().as_deref(), Some("TitleXXX"));
		tag.set_title(String::new());
		tfile.insert_tag(tag);
		file.rewind().unwrap();
		tfile.save_to(&mut file).unwrap();
		assert!(!tfile.contains_tag_type(lofty::TagType::Id3v2));
	}

	file.rewind().unwrap();

	{
		let tfile = lofty::read_from(&mut file).unwrap();

		assert_eq!(tfile.file_type(), FileType::Aiff);

		assert!(!tfile.contains_tag_type(lofty::TagType::Id3v2));
	}
}

#[test]
#[ignore] // TODO: Support writing ID3v2.3 tags
fn test_save_id3v23() {}

#[test]
#[ignore]
fn test_duplicate_id3v2() {
	// Marker test, Lofty will overwrite values in the original tag with any new values it finds in the next tag.
}

#[test]
#[ignore]
fn test_fuzzed_file1() {
	assert_eq!(
		get_filetype("tests/taglib/data/segfault.aif"),
		FileType::Aiff
	);
}

#[test]
#[ignore]
fn test_fuzzed_file2() {
	// Marker test, this file doesn't even have a valid signature. No idea how TagLib manages to read it.
}
