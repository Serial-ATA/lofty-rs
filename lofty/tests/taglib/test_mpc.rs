use crate::temp_file;
use crate::util::get_file;

use std::io::Seek;

use lofty::ape::ApeTag;
use lofty::config::{ParseOptions, WriteOptions};
use lofty::file::AudioFile;
use lofty::id3::v1::Id3v1Tag;
use lofty::musepack::{MpcFile, MpcProperties};
use lofty::probe::Probe;
use lofty::tag::{Accessor, TagExt};

#[test]
fn test_properties_sv8() {
	let f = get_file::<MpcFile>("tests/taglib/data/sv8_header.mpc");

	let MpcProperties::Sv8(properties) = f.properties() else {
		panic!("Got the wrong properties somehow")
	};

	assert_eq!(properties.version(), 8);
	assert_eq!(properties.duration().as_secs(), 1);
	assert_eq!(properties.duration().as_millis(), 1497);
	assert_eq!(properties.audio_bitrate(), 1);
	assert_eq!(properties.channels(), 2);
	assert_eq!(properties.sample_rate(), 44100);
	// TODO
	// assert_eq!(properties.sample_frames(), 66014);
}

#[test]
fn test_properties_sv7() {
	let f = get_file::<MpcFile>("tests/taglib/data/click.mpc");

	let MpcProperties::Sv7(properties) = f.properties() else {
		panic!("Got the wrong properties somehow")
	};

	assert_eq!(properties.duration().as_secs(), 0);
	assert_eq!(properties.duration().as_millis(), 40);
	assert_eq!(properties.audio_bitrate(), 318);
	assert_eq!(properties.channels(), 2);
	assert_eq!(properties.sample_rate(), 44100);
	// TODO
	// assert_eq!(properties.sample_frames(), 1760);

	assert_eq!(properties.title_gain(), 14221);
	assert_eq!(properties.title_peak(), 19848);
	assert_eq!(properties.album_gain(), 14221);
	assert_eq!(properties.album_peak(), 19848);
}

#[test]
fn test_properties_sv5() {
	let f = get_file::<MpcFile>("tests/taglib/data/sv5_header.mpc");

	let MpcProperties::Sv4to6(properties) = f.properties() else {
		panic!("Got the wrong properties somehow")
	};

	assert_eq!(properties.stream_version(), 5);
	assert_eq!(properties.duration().as_secs(), 26);
	assert_eq!(properties.duration().as_millis(), 26371);
	assert_eq!(properties.audio_bitrate(), 0);
	assert_eq!(properties.channels(), 2);
	assert_eq!(properties.sample_rate(), 44100);
	// TODO
	// assert_eq!(properties.sample_frames(), 1162944);
}

#[test]
fn test_properties_sv4() {
	let f = get_file::<MpcFile>("tests/taglib/data/sv4_header.mpc");

	let MpcProperties::Sv4to6(properties) = f.properties() else {
		panic!("Got the wrong properties somehow")
	};

	assert_eq!(properties.stream_version(), 4);
	assert_eq!(properties.duration().as_secs(), 26);
	assert_eq!(properties.duration().as_millis(), 26371);
	assert_eq!(properties.audio_bitrate(), 0);
	assert_eq!(properties.channels(), 2);
	assert_eq!(properties.sample_rate(), 44100);
	// TODO
	// assert_eq!(properties.sample_frames(), 1162944);
}

#[test]
fn test_fuzzed_file1() {
	let _ = Probe::open("tests/taglib/data/zerodiv.mpc")
		.unwrap()
		.guess_file_type()
		.unwrap();
}

#[test]
fn test_fuzzed_file2() {
	let _ = Probe::open("tests/taglib/data/infloop.mpc")
		.unwrap()
		.guess_file_type()
		.unwrap();
}

#[test]
fn test_fuzzed_file3() {
	let _ = Probe::open("tests/taglib/data/segfault.mpc")
		.unwrap()
		.guess_file_type()
		.unwrap();
}

#[test]
fn test_fuzzed_file4() {
	let _ = Probe::open("tests/taglib/data/segfault2.mpc")
		.unwrap()
		.guess_file_type()
		.unwrap();
}

#[test]
fn test_strip_and_properties() {
	let mut file = temp_file!("tests/taglib/data/click.mpc");

	{
		let mut f = MpcFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		let mut ape = ApeTag::new();
		ape.set_title(String::from("APE"));
		f.set_ape(ape);

		let mut id3v1 = Id3v1Tag::new();
		id3v1.set_title(String::from("ID3v1"));
		f.set_id3v1(id3v1);
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let mut f = MpcFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		assert_eq!(f.ape().unwrap().title().as_deref(), Some("APE"));
		f.ape_mut().unwrap().clear();
		assert_eq!(f.id3v1().unwrap().title().as_deref(), Some("ID3v1"));
		f.id3v1_mut().unwrap().clear();
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = MpcFile::read_from(&mut file, ParseOptions::new()).unwrap();

		assert!(f.ape().is_none());
		assert!(f.id3v1().is_none());
	}
}

#[test]
fn test_repeated_save() {
	let mut file = temp_file!("tests/taglib/data/click.mpc");

	{
		let mut f = MpcFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		assert!(f.ape().is_none());
		assert!(f.id3v1().is_none());

		let mut ape = ApeTag::new();
		ape.set_title(String::from("01234 56789 ABCDE FGHIJ"));
		f.set_ape(ape);

		f.save_to(&mut file, WriteOptions::default()).unwrap();
		file.rewind().unwrap();

		f.ape_mut().unwrap().set_title(String::from("0"));

		f.save_to(&mut file, WriteOptions::default()).unwrap();
		file.rewind().unwrap();

		let mut id3v1 = Id3v1Tag::new();
		id3v1.set_title(String::from("01234 56789 ABCDE FGHIJ"));
		f.set_id3v1(id3v1);
		f.ape_mut().unwrap().set_title(String::from(
			"01234 56789 ABCDE FGHIJ 01234 56789 ABCDE FGHIJ 01234 56789",
		));
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = MpcFile::read_from(&mut file, ParseOptions::new()).unwrap();
		assert!(f.ape().is_some());
		assert!(f.id3v1().is_some());
	}
}
