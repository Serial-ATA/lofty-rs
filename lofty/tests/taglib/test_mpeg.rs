use crate::temp_file;
use crate::util::get_file;

use std::fs::File;
use std::io::Seek;

use lofty::ape::ApeTag;
use lofty::config::{ParseOptions, WriteOptions};
use lofty::file::AudioFile;
use lofty::id3::v1::Id3v1Tag;
use lofty::id3::v2::{Id3v2Tag, Id3v2Version};
use lofty::mpeg::MpegFile;
use lofty::tag::Accessor;

#[test_log::test]
fn test_audio_properties_xing_header_cbr() {
	let f = get_file::<MpegFile>("tests/taglib/data/lame_cbr.mp3");

	assert_eq!(f.properties().duration().as_secs(), 1887); // TODO: Off by 9
	assert_eq!(f.properties().duration().as_millis(), 1_887_164);
	assert_eq!(f.properties().audio_bitrate(), 64);
	assert_eq!(f.properties().channels(), 1);
	assert_eq!(f.properties().sample_rate(), 44100);
	// TODO?
	// CPPUNIT_ASSERT_EQUAL(MPEG::XingHeader::Xing, f.audioProperties()->xingHeader()->type());
}

#[test_log::test]
fn test_audio_properties_xing_header_vbr() {
	let f = get_file::<MpegFile>("tests/taglib/data/lame_vbr.mp3");

	assert_eq!(f.properties().duration().as_secs(), 1887); // TODO: Off by 9
	assert_eq!(f.properties().duration().as_millis(), 1_887_164);
	assert_eq!(f.properties().audio_bitrate(), 70);
	assert_eq!(f.properties().channels(), 1);
	assert_eq!(f.properties().sample_rate(), 44100);
	// TODO?
	// CPPUNIT_ASSERT_EQUAL(MPEG::XingHeader::Xing, f.audioProperties()->xingHeader()->type());
}

#[test_log::test]
fn test_audio_properties_vbri_header() {
	let f = get_file::<MpegFile>("tests/taglib/data/rare_frames.mp3");

	assert_eq!(f.properties().duration().as_secs(), 222); // TODO: Off by 1
	assert_eq!(f.properties().duration().as_millis(), 222_198);
	assert_eq!(f.properties().audio_bitrate(), 233);
	assert_eq!(f.properties().channels(), 2);
	assert_eq!(f.properties().sample_rate(), 44100);
	// TODO?
	// CPPUNIT_ASSERT_EQUAL(MPEG::XingHeader::VBRI, f.audioProperties()->xingHeader()->type());
}

#[test_log::test]
fn test_audio_properties_no_vbr_headers() {
	let f = get_file::<MpegFile>("tests/taglib/data/bladeenc.mp3");

	assert_eq!(f.properties().duration().as_secs(), 3);
	assert_eq!(f.properties().duration().as_millis(), 3553);
	assert_eq!(f.properties().audio_bitrate(), 64);
	assert_eq!(f.properties().channels(), 1);
	assert_eq!(f.properties().sample_rate(), 44100);

	// NOTE: This test also checks the last frame of the file. That information is not saved
	//       in Lofty, and it doesn't seem too useful to expose.
}

#[test_log::test]
fn test_skip_invalid_frames_1() {
	let f = get_file::<MpegFile>("tests/taglib/data/invalid-frames1.mp3");

	assert_eq!(f.properties().duration().as_secs(), 0);
	assert_eq!(f.properties().duration().as_millis(), 392);
	assert_eq!(f.properties().audio_bitrate(), 160);
	assert_eq!(f.properties().channels(), 2);
	assert_eq!(f.properties().sample_rate(), 44100);
}

// TODO: Duration off by 27ms, as reported by FFmpeg
#[test_log::test]
#[ignore = "Different duration than TagLib and FFmpeg"]
fn test_skip_invalid_frames_2() {
	let f = get_file::<MpegFile>("tests/taglib/data/invalid-frames2.mp3");

	assert_eq!(f.properties().duration().as_secs(), 0);
	assert_eq!(f.properties().duration().as_millis(), 314);
	assert_eq!(f.properties().audio_bitrate(), 192);
	assert_eq!(f.properties().channels(), 2);
	assert_eq!(f.properties().sample_rate(), 44100);
}

// TODO: Duration off by 26ms, as reported by FFmpeg
#[test_log::test]
#[ignore = "Different duration than TagLib and FFmpeg"]
fn test_skip_invalid_frames_3() {
	let f = get_file::<MpegFile>("tests/taglib/data/invalid-frames3.mp3");

	assert_eq!(f.properties().duration().as_secs(), 0);
	assert_eq!(f.properties().duration().as_millis(), 183);
	assert_eq!(f.properties().audio_bitrate(), 362);
	assert_eq!(f.properties().channels(), 2);
	assert_eq!(f.properties().sample_rate(), 44100);
}

#[test_log::test]
fn test_version_2_duration_with_xing_header() {
	let f = get_file::<MpegFile>("tests/taglib/data/mpeg2.mp3");
	assert_eq!(f.properties().duration().as_secs(), 5387); // TODO: Off by 15
	assert_eq!(f.properties().duration().as_millis(), 5_387_285);
}

#[test_log::test]
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
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		assert_eq!(f.id3v2().unwrap().original_version(), Id3v2Version::V4);
		assert_eq!(f.id3v2().unwrap().artist().as_deref(), Some("Artist A"));
		assert_eq!(f.id3v2().unwrap().title().as_deref(), Some(xxx.as_str()));
	}
}

#[test_log::test]
#[ignore = "Marker test, Lofty does not replicate the TagLib saving API"]
fn test_save_id3v24_wrong_param() {}

#[test_log::test]
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
		f.save_to(&mut file, WriteOptions::default().use_id3v23(true))
			.unwrap();
	}
	file.rewind().unwrap();
	{
		let f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		assert_eq!(f.id3v2().unwrap().original_version(), Id3v2Version::V3);
		assert_eq!(f.id3v2().unwrap().artist().as_deref(), Some("Artist A"));
		assert_eq!(f.id3v2().unwrap().title().as_deref(), Some(xxx.as_str()));
	}
}

#[test_log::test]
fn test_duplicate_id3v2() {
	let f = get_file::<MpegFile>("tests/taglib/data/duplicate_id3v2.mp3");
	assert_eq!(f.properties().sample_rate(), 44100);
}

#[test_log::test]
fn test_fuzzed_file() {
	let mut file = File::open("tests/taglib/data/excessive_alloc.mp3").unwrap();
	assert!(MpegFile::read_from(&mut file, ParseOptions::new()).is_err())
}

#[test_log::test]
#[ignore = "Marker test, Lofty does not replicate this API. Doesn't seem useful to retain frame \
            offsets."]
fn test_frame_offset() {}

#[test_log::test]
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
		f.save_to(&mut file, WriteOptions::default()).unwrap();
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

#[test_log::test]
#[ignore = "Marker test, Lofty does not replicate the properties API"]
fn test_properties() {}

#[test_log::test]
#[ignore = "Marker test, yet another case of checking frame offsets that Lofty does not expose."]
fn test_repeated_save_1() {}

#[test_log::test]
#[ignore = "Marker test, not entirely sure what's even being tested here?"]
fn test_repeated_save_2() {}

#[test_log::test]
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
			f.save_to(&mut file, WriteOptions::default()).unwrap();
		}
		file.rewind().unwrap();
		{
			f.ape_mut().unwrap().set_title(String::from("0"));
			f.save_to(&mut file, WriteOptions::default()).unwrap();
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
			f.save_to(&mut file, WriteOptions::default()).unwrap();
		}
	}
	file.rewind().unwrap();
	{
		let f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		assert!(f.ape().is_some());
		assert!(f.id3v1().is_some());
	}
}

#[test_log::test]
#[ignore = "Marker test, Lofty accepts empty strings as valid values"]
fn test_empty_id3v2() {}

#[test_log::test]
#[ignore = "Marker test, Lofty accepts empty strings as valid values"]
fn test_empty_id3v1() {}

#[test_log::test]
#[ignore = "Marker test, Lofty accepts empty strings as valid values"]
fn test_empty_ape() {}

#[test_log::test]
fn test_ignore_garbage() {
	let mut file = temp_file!("tests/taglib/data/garbage.mp3");

	{
		let mut f =
			MpegFile::read_from(&mut file, ParseOptions::new().max_junk_bytes(3000)).unwrap();
		file.rewind().unwrap();
		assert!(f.id3v2().is_some());

		assert_eq!(f.id3v2().unwrap().title().as_deref(), Some("Title A"));
		f.id3v2_mut().unwrap().set_title(String::from("Title B"));
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = MpegFile::read_from(&mut file, ParseOptions::new().max_junk_bytes(3000)).unwrap();
		assert!(f.id3v2().is_some());
		assert_eq!(f.id3v2().unwrap().title().as_deref(), Some("Title B"));
	}
}
