use lofty::config::ParseOptions;
use lofty::file::FileType;
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::TagType;

use std::io::Seek;

#[test_log::test]
fn read() {
	// Here we have an AAC file with an ID3v2, and an ID3v1 tag
	let file = Probe::open("tests/files/assets/minimal/full_test.aac")
		.unwrap()
		.options(ParseOptions::new().read_properties(false))
		.read()
		.unwrap();

	assert_eq!(file.file_type(), FileType::Aac);

	// Verify the ID3v2 tag first
	crate::util::verify_artist(&file, TagType::Id3v2, "Foo artist", 1);

	// Now verify ID3v1
	crate::util::verify_artist(&file, TagType::Id3v1, "Bar artist", 1);
}

#[test_log::test]
fn read_with_junk_bytes_between_frames() {
	// Read a file that includes an ID3v2.3 data block followed by four bytes of junk data (0x20)

	// This is the same test as MP3, but it uses the same byte skipping logic, so it should be tested
	// here too :).
	let file = Probe::open("tests/files/assets/junk_between_id3_and_adts.aac")
		.unwrap()
		.read()
		.unwrap();

	// note that the file contains ID3v2 and ID3v1 data
	assert_eq!(file.file_type(), FileType::Aac);

	let id3v2_tag = &file.tags()[0];
	assert_eq!(id3v2_tag.artist().as_deref(), Some("artist test"));
	assert_eq!(id3v2_tag.album().as_deref(), Some("album test"));
	assert_eq!(id3v2_tag.title().as_deref(), Some("title test"));
	assert_eq!(
		id3v2_tag.get_string(ItemKey::EncoderSettings),
		Some("Lavf58.62.100")
	);

	let id3v1_tag = &file.tags()[1];
	assert_eq!(id3v1_tag.artist().as_deref(), Some("artist test"));
	assert_eq!(id3v1_tag.album().as_deref(), Some("album test"));
	assert_eq!(id3v1_tag.title().as_deref(), Some("title test"));
}

#[test_log::test]
fn write() {
	let mut tagged_file = crate::util::read("tests/files/assets/minimal/full_test.aac");

	assert_eq!(tagged_file.file_type(), FileType::Aac);

	// ID3v2
	crate::util::set_artist(
		&mut tagged_file,
		TagType::Id3v2,
		"Foo artist",
		"Bar artist",
		1,
	);

	// ID3v1
	crate::util::set_artist(
		&mut tagged_file,
		TagType::Id3v1,
		"Bar artist",
		"Baz artist",
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
		"Foo artist",
		1,
	);

	crate::util::set_artist(
		&mut tagged_file,
		TagType::Id3v1,
		"Baz artist",
		"Bar artist",
		1,
	);
}

#[test_log::test]
fn remove_id3v2() {
	crate::util::remove_tag_test("tests/files/assets/minimal/full_test.aac", TagType::Id3v2);
}

#[test_log::test]
fn remove_id3v1() {
	crate::util::remove_tag_test("tests/files/assets/minimal/full_test.aac", TagType::Id3v1);
}

#[test_log::test]
fn read_no_properties() {
	crate::util::no_properties_test("tests/files/assets/minimal/full_test.aac");
}

#[test_log::test]
fn read_no_tags() {
	crate::util::no_tag_test("tests/files/assets/minimal/full_test.aac", None);
}
