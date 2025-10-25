use lofty::config::ParseOptions;
use lofty::file::FileType;
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::TagType;

use std::io::Seek;

#[test_log::test]
fn read() {
	// This file contains an ilst atom
	let file = Probe::open("tests/files/assets/minimal/m4a_codec_aac.m4a")
		.unwrap()
		.options(ParseOptions::new().read_properties(false))
		.read()
		.unwrap();

	assert_eq!(file.file_type(), FileType::Mp4);

	// Verify the ilst tag
	crate::util::verify_artist(&file, TagType::Mp4Ilst, "Foo artist", 1);
}

#[test_log::test]
fn write() {
	let mut tagged_file = crate::util::read("tests/files/assets/minimal/m4a_codec_aac.m4a");

	assert_eq!(tagged_file.file_type(), FileType::Mp4);

	// ilst
	crate::util::set_artist(
		&mut tagged_file,
		TagType::Mp4Ilst,
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
		TagType::Mp4Ilst,
		"Bar artist",
		"Foo artist",
		1,
	);
}

#[test_log::test]
fn remove() {
	crate::util::remove_tag_test(
		"tests/files/assets/minimal/m4a_codec_aac.m4a",
		TagType::Mp4Ilst,
	);
}

#[test_log::test]
fn read_no_properties() {
	crate::util::no_properties_test("tests/files/assets/minimal/m4a_codec_aac.m4a");
}

#[test_log::test]
fn read_no_tags() {
	crate::util::no_tag_test("tests/files/assets/minimal/m4a_codec_aac.m4a", None);
}
