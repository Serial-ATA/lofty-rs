use lofty::config::ParseOptions;
use lofty::file::FileType;
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::TagType;

use std::io::Seek;

#[test_log::test]
fn read() {
	// Here we have an APE file with an ID3v2, ID3v1, and an APEv2 tag
	let file = Probe::open("tests/files/assets/minimal/full_test.ape")
		.unwrap()
		.options(ParseOptions::new().read_properties(false))
		.read()
		.unwrap();

	assert_eq!(file.file_type(), FileType::Ape);

	// Verify the APEv2 tag first
	crate::util::verify_artist(&file, TagType::Ape, "Foo artist", 1);

	// Now verify ID3v1
	crate::util::verify_artist(&file, TagType::Id3v1, "Bar artist", 1);

	// Finally, verify ID3v2
	crate::util::verify_artist(&file, TagType::Id3v2, "Baz artist", 1);
}

#[test_log::test]
fn write() {
	// We don't write an ID3v2 tag here since it's against the spec
	let mut tagged_file = crate::util::read("tests/files/assets/minimal/full_test.ape");

	assert_eq!(tagged_file.file_type(), FileType::Ape);

	// APEv2
	crate::util::set_artist(
		&mut tagged_file,
		TagType::Ape,
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
		TagType::Ape,
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
fn remove_ape() {
	crate::util::remove_tag_test("tests/files/assets/minimal/full_test.ape", TagType::Ape);
}

#[test_log::test]
fn remove_id3v1() {
	crate::util::remove_tag_test("tests/files/assets/minimal/full_test.ape", TagType::Id3v1);
}

#[test_log::test]
fn remove_id3v2() {
	crate::util::remove_tag_test("tests/files/assets/minimal/full_test.ape", TagType::Id3v2);
}

#[test_log::test]
fn read_no_properties() {
	crate::util::no_properties_test("tests/files/assets/minimal/full_test.ape");
}

#[test_log::test]
fn read_no_tags() {
	crate::util::no_tag_test("tests/files/assets/minimal/full_test.ape", None);
}
