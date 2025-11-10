use crate::util::get_file;
use crate::{assert_delta, temp_file};

use std::io::Seek;

use lofty::config::{ParseOptions, WriteOptions};
use lofty::file::{AudioFile, FileType};
use lofty::id3::v2::{Id3v2Tag, Id3v2Version};
use lofty::iff::aiff::{AiffCompressionType, AiffFile};
use lofty::probe::Probe;
use lofty::tag::{Accessor, TagType};

#[test_log::test]
fn test_aiff_properties() {
	let file = get_file::<AiffFile>("tests/taglib/data/empty.aiff");

	let properties = file.properties();
	assert_eq!(properties.duration().as_secs(), 0);
	assert_delta!(properties.duration().as_millis(), 67, 1);
	assert_delta!(properties.audio_bitrate(), 706, 1);
	assert_eq!(properties.sample_rate(), 44100);
	assert_eq!(properties.channels(), 1);
	assert_eq!(properties.sample_size(), 16);
	// TODO: get those options in lofty
	// CPPUNIT_ASSERT_EQUAL(2941U, f.audioProperties()->sampleFrames());
	assert!(properties.compression_type().is_none());
}

#[test_log::test]
fn test_aifc_properties() {
	let file = get_file::<AiffFile>("tests/taglib/data/alaw.aifc");

	let properties = file.properties();
	assert_eq!(properties.duration().as_secs(), 0);
	assert_delta!(properties.duration().as_millis(), 37, 1);
	assert_eq!(properties.audio_bitrate(), 355);
	assert_eq!(properties.sample_rate(), 44100);
	assert_eq!(properties.channels(), 1);
	assert_eq!(properties.sample_size(), 16);
	// TODO: get those options in lofty
	// CPPUNIT_ASSERT_EQUAL(1622U, f.audioProperties()->sampleFrames());
	assert!(properties.compression_type().is_some());
	assert_eq!(
		properties.compression_type().unwrap().clone(),
		AiffCompressionType::ALAW
	);
	// NOTE: The file's compression name is actually "SGI CCITT G.711 A-law"
	//
	// We have a hardcoded value for any of the concrete AiffCompressionType variants, as the messages
	// are more or less standardized. This is not that big of a deal, especially as many encoders choose
	// not to even write a compression name in the first place.
	assert_eq!(
		properties.compression_type().unwrap().compression_name(),
		"CCITT G.711 A-law"
	);
}

#[test_log::test]
fn test_save_id3v2() {
	let mut file = temp_file!("tests/taglib/data/empty.aiff");

	{
		let mut tfile = AiffFile::read_from(&mut file, ParseOptions::new()).unwrap();

		assert!(tfile.id3v2().is_none());

		let mut id3v2 = Id3v2Tag::new();
		id3v2.set_title("TitleXXX".to_string());
		tfile.set_id3v2(id3v2);
		file.rewind().unwrap();
		tfile.save_to(&mut file, WriteOptions::default()).unwrap();
		assert!(tfile.contains_tag_type(TagType::Id3v2));
	}

	file.rewind().unwrap();

	{
		let mut tfile = AiffFile::read_from(&mut file, ParseOptions::new()).unwrap();

		let mut id3v2 = tfile.id3v2().unwrap().to_owned();
		assert_eq!(id3v2.title().as_deref(), Some("TitleXXX"));
		// NOTE: TagLib sets an empty title, which will implicitly remove it from the tag. Lofty will allow empty tag items to exist.
		//       What's important is that these are equivalent in behavior.
		id3v2.remove_title();
		tfile.set_id3v2(id3v2);
		file.rewind().unwrap();
		tfile.save_to(&mut file, WriteOptions::default()).unwrap();
	}

	file.rewind().unwrap();

	{
		let tfile = AiffFile::read_from(&mut file, ParseOptions::new()).unwrap();
		assert!(!tfile.contains_tag_type(TagType::Id3v2));
	}
}

#[test_log::test]
fn test_save_id3v23() {
	let mut file = temp_file!("tests/taglib/data/empty.aiff");

	let xxx = "X".repeat(254);
	{
		let mut tfile = AiffFile::read_from(&mut file, ParseOptions::new()).unwrap();

		assert!(tfile.id3v2().is_none());

		let mut id3v2 = Id3v2Tag::new();
		id3v2.set_title(xxx.clone());
		id3v2.set_artist(String::from("Artist A"));
		tfile.set_id3v2(id3v2);
		file.rewind().unwrap();
		tfile
			.save_to(&mut file, WriteOptions::default().use_id3v23(true))
			.unwrap();
		assert!(tfile.contains_tag_type(TagType::Id3v2));
	}
	file.rewind().unwrap();
	{
		let tfile = AiffFile::read_from(&mut file, ParseOptions::new()).unwrap();
		let id3v2 = tfile.id3v2().unwrap().to_owned();
		assert_eq!(id3v2.original_version(), Id3v2Version::V3);
		assert_eq!(id3v2.artist().as_deref(), Some("Artist A"));
		assert_eq!(id3v2.title().as_deref(), Some(&*xxx));
	}
}

#[test_log::test]
#[ignore = "Marker test, Lofty will overwrite values in the original tag with any new values it \
            finds in the next tag."]
fn test_duplicate_id3v2() {}

#[test_log::test]
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

#[test_log::test]
#[ignore = "Marker test, this file doesn't even have a valid signature. No idea how TagLib manages \
            to read it."]
fn test_fuzzed_file2() {}
