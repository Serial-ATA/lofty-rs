use crate::temp_file;

use std::fs::File;
use std::io::Seek;

use lofty::ape::ApeTag;
use lofty::id3::v1::Id3v1Tag;
use lofty::id3::v2::{Id3v2Tag, Id3v2Version};
use lofty::mpeg::MpegFile;
use lofty::{Accessor, AudioFile, ParseOptions};

#[test]
#[ignore]
fn test_audio_properties_xing_header_cbr() {
	let mut file = File::open("tests/taglib/data/lame_cbr.mp3").unwrap();
	let f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();

	assert_eq!(f.properties().duration().as_secs(), 1887); // TODO: Off by 9
	assert_eq!(f.properties().duration().as_millis(), 1887164);
	assert_eq!(f.properties().audio_bitrate(), 64);
	assert_eq!(f.properties().channels(), 1);
	assert_eq!(f.properties().sample_rate(), 44100);
	// TODO?
	// CPPUNIT_ASSERT_EQUAL(MPEG::XingHeader::Xing, f.audioProperties()->xingHeader()->type());
}

#[test]
#[ignore]
fn test_audio_properties_xing_header_vbr() {
	let mut file = File::open("tests/taglib/data/lame_vbr.mp3").unwrap();
	let f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();

	assert_eq!(f.properties().duration().as_secs(), 1887); // TODO: Off by 9
	assert_eq!(f.properties().duration().as_millis(), 1887164);
	assert_eq!(f.properties().audio_bitrate(), 70);
	assert_eq!(f.properties().channels(), 1);
	assert_eq!(f.properties().sample_rate(), 44100);
	// TODO?
	// CPPUNIT_ASSERT_EQUAL(MPEG::XingHeader::Xing, f.audioProperties()->xingHeader()->type());
}

#[test]
#[ignore]
fn test_audio_properties_vbri_header() {
	let mut file = File::open("tests/taglib/data/rare_frames.mp3").unwrap();
	let f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();

	assert_eq!(f.properties().duration().as_secs(), 222); // TODO: Off by 1
	assert_eq!(f.properties().duration().as_millis(), 222198);
	assert_eq!(f.properties().audio_bitrate(), 233);
	assert_eq!(f.properties().channels(), 2);
	assert_eq!(f.properties().sample_rate(), 44100);
	// TODO?
	// CPPUNIT_ASSERT_EQUAL(MPEG::XingHeader::VBRI, f.audioProperties()->xingHeader()->type());
}

#[test]
#[ignore]
fn test_audio_properties_no_vbr_headers() {
	let mut file = File::open("tests/taglib/data/bladeenc.mp3").unwrap();
	let f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();

	assert_eq!(f.properties().duration().as_secs(), 3); // Off by 1
	assert_eq!(f.properties().duration().as_millis(), 3553);
	assert_eq!(f.properties().audio_bitrate(), 64);
	assert_eq!(f.properties().channels(), 1);
	assert_eq!(f.properties().sample_rate(), 44100);

	// NOTE: This test also checks the last frame of the file. That information is not saved
	//       in Lofty, and it doesn't seem too useful to expose.
}

#[test]
fn test_skip_invalid_frames_1() {
	let mut file = File::open("tests/taglib/data/invalid-frames1.mp3").unwrap();
	let f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();

	assert_eq!(f.properties().duration().as_secs(), 0);
	assert_eq!(f.properties().duration().as_millis(), 392);
	assert_eq!(f.properties().audio_bitrate(), 160);
	assert_eq!(f.properties().channels(), 2);
	assert_eq!(f.properties().sample_rate(), 44100);
}

#[test]
#[ignore]
fn test_skip_invalid_frames_2() {
	let mut file = File::open("tests/taglib/data/invalid-frames2.mp3").unwrap();
	let f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();

	assert_eq!(f.properties().duration().as_secs(), 0);
	assert_eq!(f.properties().duration().as_millis(), 314); // TODO: Off by 79
	assert_eq!(f.properties().audio_bitrate(), 192);
	assert_eq!(f.properties().channels(), 2);
	assert_eq!(f.properties().sample_rate(), 44100);
}

#[test]
#[ignore]
fn test_skip_invalid_frames_3() {
	let mut file = File::open("tests/taglib/data/invalid-frames3.mp3").unwrap();
	let f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();

	assert_eq!(f.properties().duration().as_secs(), 0);
	assert_eq!(f.properties().duration().as_millis(), 183); // TODO: Off by 26
	assert_eq!(f.properties().audio_bitrate(), 320);
	assert_eq!(f.properties().channels(), 2);
	assert_eq!(f.properties().sample_rate(), 44100);
}

#[test]
#[ignore]
fn test_version_2_duration_with_xing_header() {
	let mut file = File::open("tests/taglib/data/mpeg2.mp3").unwrap();
	let f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
	assert_eq!(f.properties().duration().as_secs(), 5387); // TODO: Off by 15
	assert_eq!(f.properties().duration().as_millis(), 5387285);
}

#[test]
fn test_save_id3v24() {
	let mut file = temp_file!("tests/taglib/data/xing.mp3");

	let xxx = "X".repeat(254);
	{
		let mut f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();
		assert!(f.id3v2().is_none());

		let mut tag = Id3v2Tag::default();
		tag.set_title(xxx.clone());
		tag.set_artist(String::from("Artist A"));
		f.set_id3v2(tag);
		f.save_to(&mut file).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		assert_eq!(f.id3v2().unwrap().original_version(), Id3v2Version::V4);
		assert_eq!(f.id3v2().unwrap().artist().as_deref(), Some("Artist A"));
		assert_eq!(f.id3v2().unwrap().title().as_deref(), Some(xxx.as_str()));
	}
}

#[test]
#[ignore]
fn test_save_id3v24_wrong_param() {
	// Marker test, Lofty does not replicate the TagLib saving API
}

#[test]
#[ignore] // TODO: We don't yet support writing an ID3v23 tag (#62)
fn test_save_id3v23() {
	let mut file = temp_file!("tests/taglib/data/xing.mp3");

	let xxx = "X".repeat(254);
	{
		let mut f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();
		assert!(f.id3v2().is_none());

		let mut tag = Id3v2Tag::default();
		tag.set_title(xxx.clone());
		tag.set_artist(String::from("Artist A"));
		f.set_id3v2(tag);
		f.save_to(&mut file).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		assert_eq!(f.id3v2().unwrap().original_version(), Id3v2Version::V3);
		assert_eq!(f.id3v2().unwrap().artist().as_deref(), Some("Artist A"));
		assert_eq!(f.id3v2().unwrap().title().as_deref(), Some(xxx.as_str()));
	}
}

#[test]
fn test_duplicate_id3v2() {
	let mut file = File::open("tests/taglib/data/duplicate_id3v2.mp3").unwrap();
	let f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
	assert_eq!(f.properties().sample_rate(), 44100);
}

#[test]
fn test_fuzzed_file() {
	let mut file = File::open("tests/taglib/data/excessive_alloc.mp3").unwrap();
	let _ = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
}

#[test]
#[ignore]
fn test_frame_offset() {
	// Marker test, Lofty does not replicate this API. Doesn't seem useful to retain frame offsets.
}

#[test]
fn test_strip_and_properties() {
	let mut file = temp_file!("tests/taglib/data/xing.mp3");

	{
		let mut f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		let mut id3v2 = Id3v2Tag::default();
		id3v2.set_title(String::from("ID3v2"));
		f.set_id3v2(id3v2);
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
		let mut f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		assert_eq!(f.id3v2().unwrap().title().as_deref(), Some("ID3v2"));
		f.remove_id3v2();
		assert_eq!(f.ape().unwrap().title().as_deref(), Some("APE"));
		f.remove_ape();
		assert_eq!(f.id3v1().unwrap().title().as_deref(), Some("ID3v1"));
		f.remove_id3v1();
		assert!(!f.contains_tag());
	}
}

#[test]
fn test_properties() {}

#[test]
#[ignore]
fn test_repeated_save_1() {
	// Marker test, yet another case of checking frame offsets that Lofty does not expose.
}

#[test]
#[ignore]
fn test_repeated_save_2() {
	// Marker test, not entirely sure what's even being tested here?
}

#[test]
fn test_repeated_save_3() {
	let mut file = temp_file!("tests/taglib/data/xing.mp3");

	{
		let mut f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();
		assert!(f.ape().is_none());
		assert!(f.id3v1().is_none());

		{
			let mut ape = ApeTag::default();
			ape.set_title(String::from("01234 56789 ABCDE FGHIJ"));
			f.set_ape(ape);
			f.save_to(&mut file).unwrap();
		}
		file.rewind().unwrap();
		{
			f.ape_mut().unwrap().set_title(String::from("0"));
			f.save_to(&mut file).unwrap();
		}
		{
			let mut id3v1 = Id3v1Tag::default();
			id3v1.set_title(String::from("01234 56789 ABCDE FGHIJ"));
			f.set_id3v1(id3v1);
		}
		file.rewind().unwrap();
		{
			f.ape_mut().unwrap().set_title(String::from(
				"01234 56789 ABCDE FGHIJ 01234 56789 ABCDE FGHIJ 01234 56789",
			));
			f.save_to(&mut file).unwrap();
		}
	}
	file.rewind().unwrap();
	{
		let f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		assert!(f.ape().is_some());
		assert!(f.id3v1().is_some());
	}
}

#[test]
#[ignore]
fn test_empty_id3v2() {
	// Marker test, Lofty accepts empty strings as valid values
}

#[test]
#[ignore]
fn test_empty_id3v1() {
	// Marker test, Lofty accepts empty strings as valid values
}

#[test]
#[ignore]
fn test_empty_ape() {
	// Marker test, Lofty accepts empty strings as valid values
}

#[test]
#[ignore] // TODO: We can't find an ID3v2 tag after saving with garbage
fn test_ignore_garbage() {
	let mut file = temp_file!("tests/taglib/data/garbage.mp3");

	{
		let mut f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();
		assert!(f.id3v2().is_some());

		assert_eq!(f.id3v2().unwrap().title().as_deref(), Some("Title A"));
		f.id3v2_mut().unwrap().set_title(String::from("Title B"));
		f.save_to(&mut file).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		assert!(f.id3v2().is_some());
		assert_eq!(f.id3v2().unwrap().title().as_deref(), Some("Title B"));
	}
}
