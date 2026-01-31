use lofty::config::ParseOptions;
use lofty::file::FileType;
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::TagType;

use std::io::Seek;

// NOTE: These tests require test audio files that don't exist yet.
// You'll need to create or obtain:
// - tests/files/assets/minimal/full_test.dff (DFF file with ID3v2 and DffText tags)

#[test_log::test]
fn read() {
	// Here we have a DFF file with both an ID3v2 chunk and DFF text chunks (DIIN)
	let file = Probe::open("tests/files/assets/minimal/full_test.dff")
		.unwrap()
		.options(ParseOptions::new().read_properties(false))
		.read()
		.unwrap();

	assert_eq!(file.file_type(), FileType::Dff);

	// Verify the ID3v2 tag first
	crate::util::verify_artist(&file, TagType::Id3v2, "Foo artist", 1);

	// Now verify the DFF text chunks (has artist, title, and 2 comments = 4 items)
	crate::util::verify_artist(&file, TagType::DffText, "Bar artist", 4);

	// Verify comment is present
	let dff_tag = file.tag(TagType::DffText).unwrap();
	assert_eq!(dff_tag.comment().as_deref(), Some("This is a test comment"));
}

#[test_log::test]
fn write() {
	let mut tagged_file = crate::util::read("tests/files/assets/minimal/full_test.dff");

	assert_eq!(tagged_file.file_type(), FileType::Dff);

	// ID3v2
	crate::util::set_artist(
		&mut tagged_file,
		TagType::Id3v2,
		"Foo artist",
		"Bar artist",
		1,
	);

	// DFF text chunks (when writing is implemented)
	// crate::util::set_artist(
	// 	&mut tagged_file,
	// 	TagType::DffText,
	// 	"Bar artist",
	// 	"Baz artist",
	// 	1,
	// );

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
	crate::util::remove_tag_test("tests/files/assets/minimal/full_test.dff", TagType::Id3v2);
}

#[test_log::test]
fn remove_dff_text() {
	crate::util::remove_tag_test("tests/files/assets/minimal/full_test.dff", TagType::DffText);
}

#[test_log::test]
fn read_properties() {
	let file = Probe::open("tests/files/assets/minimal/full_test.dff")
		.unwrap()
		.read()
		.unwrap();

	assert_eq!(file.file_type(), FileType::Dff);

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
}
