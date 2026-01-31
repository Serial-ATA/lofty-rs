use lofty::config::ParseOptions;
use lofty::file::FileType;
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::TagType;

use std::io::Seek;

// NOTE: These tests require test audio files that don't exist yet.
// You'll need to create or obtain:
// - tests/files/assets/minimal/full_test.dsf (DSF file with ID3v2 tag)

#[test_log::test]
fn read() {
	// Here we have a DSF file with an ID3v2 tag
	let file = Probe::open("tests/files/assets/minimal/full_test.dsf")
		.unwrap()
		.options(ParseOptions::new().read_properties(false))
		.read()
		.unwrap();

	assert_eq!(file.file_type(), FileType::Dsf);

	// Verify the ID3v2 tag
	crate::util::verify_artist(&file, TagType::Id3v2, "Foo artist", 1);
}

#[test_log::test]
fn write() {
	let mut tagged_file = crate::util::read("tests/files/assets/minimal/full_test.dsf");

	assert_eq!(tagged_file.file_type(), FileType::Dsf);

	// ID3v2
	crate::util::set_artist(
		&mut tagged_file,
		TagType::Id3v2,
		"Foo artist",
		"Bar artist",
		1,
	);

	// Now reread the file
	let mut file = tagged_file.into_inner();
	file.rewind().unwrap();

	let mut tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read_bound()
		.unwrap();

	crate::util::set_artist(
		&mut tagged_file,
		TagType::Id3v2,
		"Bar artist",
		"Baz artist",
		1,
	);
}

#[test_log::test]
fn remove_id3v2() {
	crate::util::remove_tag_test("tests/files/assets/minimal/full_test.dsf", TagType::Id3v2);
}

#[test_log::test]
fn read_properties() {
	let file = Probe::open("tests/files/assets/minimal/full_test.dsf")
		.unwrap()
		.read()
		.unwrap();

	assert_eq!(file.file_type(), FileType::Dsf);

	let properties = file.properties();

	// Verify duration is valid
	assert!(properties.duration().as_millis() > 0);

	// Verify sample rate
	assert!(properties.sample_rate().is_some());
	let sample_rate = properties.sample_rate().unwrap();
	assert!(sample_rate > 0);

	// Verify bit depth (DSD is 1 bit)
	assert_eq!(properties.bit_depth(), Some(1));

	// Verify channels
	assert!(properties.channels().is_some());
	let channels = properties.channels().unwrap();
	assert!((1..=6).contains(&channels));

	// Verify bitrates
	assert!(properties.overall_bitrate().is_some());
	assert!(properties.audio_bitrate().is_some());

	// Verify channel mask (DSF should have this)
	assert!(properties.channel_mask().is_some());
}
