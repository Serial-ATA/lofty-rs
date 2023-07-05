use crate::temp_file;

use std::io::Seek;

use lofty::ape::ApeTag;
use lofty::id3::v1::Id3v1Tag;
use lofty::wavpack::WavPackFile;
use lofty::{Accessor, AudioFile, ParseOptions};

#[test]
fn test_no_length_properties() {
	let mut file = temp_file!("tests/taglib/data/no_length.wv");
	let f = WavPackFile::read_from(&mut file, ParseOptions::new()).unwrap();
	assert_eq!(f.properties().duration().as_secs(), 3);
	assert_eq!(f.properties().duration().as_millis(), 3705);
	assert_eq!(f.properties().audio_bitrate(), 1);
	assert_eq!(f.properties().channels(), 2);
	assert_eq!(f.properties().bit_depth(), 16);
	assert_eq!(f.properties().is_lossless(), true);
	assert_eq!(f.properties().sample_rate(), 44100);
	// TODO: CPPUNIT_ASSERT_EQUAL(163392U, f.audioProperties()->sampleFrames());
	assert_eq!(f.properties().version(), 1031);
}

#[test]
fn test_multi_channel_properties() {
	let mut file = temp_file!("tests/taglib/data/four_channels.wv");
	let f = WavPackFile::read_from(&mut file, ParseOptions::new()).unwrap();
	assert_eq!(f.properties().duration().as_secs(), 3);
	assert_eq!(f.properties().duration().as_millis(), 3833);
	assert_eq!(f.properties().audio_bitrate(), 112);
	assert_eq!(f.properties().channels(), 4);
	assert_eq!(f.properties().bit_depth(), 16);
	assert_eq!(f.properties().is_lossless(), false);
	assert_eq!(f.properties().sample_rate(), 44100);
	// TODO: CPPUNIT_ASSERT_EQUAL(169031U, f.audioProperties()->sampleFrames());
	assert_eq!(f.properties().version(), 1031);
}

#[test]
fn test_dsd_stereo_properties() {
	let mut file = temp_file!("tests/taglib/data/dsd_stereo.wv");
	let f = WavPackFile::read_from(&mut file, ParseOptions::new()).unwrap();
	assert_eq!(f.properties().duration().as_secs(), 0);
	assert_eq!(f.properties().duration().as_millis(), 200);
	assert_eq!(f.properties().audio_bitrate(), 2096);
	assert_eq!(f.properties().channels(), 2);
	assert_eq!(f.properties().bit_depth(), 8);
	assert_eq!(f.properties().is_lossless(), true);
	assert_eq!(f.properties().sample_rate(), 352800);
	// TODO: CPPUNIT_ASSERT_EQUAL(70560U, f.audioProperties()->sampleFrames());
	assert_eq!(f.properties().version(), 1040);
}

#[test]
fn test_non_standard_rate_properties() {
	let mut file = temp_file!("tests/taglib/data/non_standard_rate.wv");
	let f = WavPackFile::read_from(&mut file, ParseOptions::new()).unwrap();
	assert_eq!(f.properties().duration().as_secs(), 3);
	assert_eq!(f.properties().duration().as_millis(), 3675);
	assert_eq!(f.properties().audio_bitrate(), 0);
	assert_eq!(f.properties().channels(), 2);
	assert_eq!(f.properties().bit_depth(), 16);
	assert_eq!(f.properties().is_lossless(), true);
	assert_eq!(f.properties().sample_rate(), 1000);
	// TODO: CPPUNIT_ASSERT_EQUAL(3675U, f.audioProperties()->sampleFrames());
	assert_eq!(f.properties().version(), 1040);
}

#[test]
fn test_tagged_properties() {
	let mut file = temp_file!("tests/taglib/data/tagged.wv");
	let f = WavPackFile::read_from(&mut file, ParseOptions::new()).unwrap();
	assert_eq!(f.properties().duration().as_secs(), 3);
	assert_eq!(f.properties().duration().as_millis(), 3550);
	assert_eq!(f.properties().audio_bitrate(), 172);
	assert_eq!(f.properties().channels(), 2);
	assert_eq!(f.properties().bit_depth(), 16);
	assert_eq!(f.properties().is_lossless(), false);
	assert_eq!(f.properties().sample_rate(), 44100);
	// TODO: CPPUNIT_ASSERT_EQUAL(156556U, f.audioProperties()->sampleFrames());
	assert_eq!(f.properties().version(), 1031);
}

#[test]
fn test_fuzzed_file() {
	let mut file = temp_file!("tests/taglib/data/infloop.wv");
	let _ = WavPackFile::read_from(&mut file, ParseOptions::new()).unwrap();
}

#[test]
fn test_strip_and_properties() {
	let mut file = temp_file!("tests/taglib/data/click.wv");

	{
		let mut f = WavPackFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		let mut ape = ApeTag::default();
		ape.set_title(String::from("APE"));
		f.set_ape(ape);

		let mut id3v1 = Id3v1Tag::default();
		id3v1.set_title(String::from("ID3v1"));
		f.set_id3v1(id3v1);

		f.save_to(&mut file).unwrap();
	}
	file.rewind().unwrap();
	{
		// NOTE: This is not the same as the TagLib test.
		//       Their test checks the first "TITLE" which changes when tags are stripped.
		let mut f = WavPackFile::read_from(&mut file, ParseOptions::new()).unwrap();
		assert_eq!(f.ape().unwrap().title().as_deref(), Some("APE"));
		f.remove_ape();
		assert_eq!(f.id3v1().unwrap().title.as_deref(), Some("ID3v1"));
		f.remove_id3v1();
		assert!(!f.contains_tag());
	}
}

#[test]
fn test_repeated_save() {
	let mut file = temp_file!("tests/taglib/data/click.wv");

	{
		let mut f = WavPackFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();
		assert!(f.ape().is_none());
		assert!(f.id3v1().is_none());

		let mut ape = ApeTag::default();
		ape.set_title(String::from("01234 56789 ABCDE FGHIJ"));
		f.set_ape(ape);
		f.save_to(&mut file).unwrap();
		file.rewind().unwrap();

		f.ape_mut().unwrap().set_title(String::from("0"));
		f.save_to(&mut file).unwrap();
		file.rewind().unwrap();

		let mut id3v1 = Id3v1Tag::default();
		id3v1.set_title(String::from("01234 56789 ABCDE FGHIJ"));
		f.set_id3v1(id3v1);
		f.ape_mut().unwrap().set_title(String::from(
			"01234 56789 ABCDE FGHIJ 01234 56789 ABCDE FGHIJ 01234 56789",
		));
		f.save_to(&mut file).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = WavPackFile::read_from(&mut file, ParseOptions::new()).unwrap();
		assert!(f.ape().is_some());
		assert!(f.id3v1().is_some());
	}
}
