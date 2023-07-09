use crate::util::get_file;
use crate::{assert_delta, temp_file};

use std::io::Seek;

use lofty::id3::v2::Id3v2Tag;
use lofty::iff::aiff::AiffFile;
use lofty::{Accessor, AudioFile, FileType, ParseOptions, Probe};

#[test]
#[ignore]
fn test_aiff_properties() {
	let file = get_file::<AiffFile>("tests/taglib/data/empty.aiff");

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
	let file = get_file::<AiffFile>("tests/taglib/data/alaw.aifc");

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
		let mut tfile = AiffFile::read_from(&mut file, ParseOptions::new()).unwrap();

		assert!(tfile.id3v2().is_none());

		let mut id3v2 = Id3v2Tag::new();
		id3v2.set_title("TitleXXX".to_string());
		tfile.set_id3v2(id3v2);
		file.rewind().unwrap();
		tfile.save_to(&mut file).unwrap();
		assert!(tfile.contains_tag_type(lofty::TagType::Id3v2));
	}

	file.rewind().unwrap();

	{
		let mut tfile = AiffFile::read_from(&mut file, ParseOptions::new()).unwrap();

		let mut id3v2 = tfile.id3v2().unwrap().to_owned();
		assert_eq!(id3v2.title().as_deref(), Some("TitleXXX"));
		id3v2.set_title(String::new());
		tfile.set_id3v2(id3v2);
		file.rewind().unwrap();
		tfile.save_to(&mut file).unwrap();
		assert!(!tfile.contains_tag_type(lofty::TagType::Id3v2));
	}

	file.rewind().unwrap();

	{
		let tfile = AiffFile::read_from(&mut file, ParseOptions::new()).unwrap();
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
fn test_fuzzed_file1() {
	assert_eq!(
		Probe::open("tests/taglib/data/segfault.aif")
			.unwrap()
			.guess_file_type()
			.unwrap()
			.file_type(),
		Some(FileType::Aiff)
	);
}

#[test]
#[ignore]
fn test_fuzzed_file2() {
	// Marker test, this file doesn't even have a valid signature. No idea how TagLib manages to read it.
}
