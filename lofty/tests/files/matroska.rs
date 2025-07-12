use crate::{set_artist, temp_file, verify_artist};
use lofty::config::ParseOptions;
use lofty::file::FileType;
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::TagType;

use std::io::Seek;

#[test_log::test]
fn read() {
	// This file contains a tags element
	let file = Probe::open("tests/files/assets/minimal/full_test.mka")
		.unwrap()
		.options(ParseOptions::new())
		.read()
		.unwrap();

	assert_eq!(file.file_type(), FileType::Ebml);

	// Verify the tag
	crate::verify_artist!(file, primary_tag, "Foo artist", 1);
}

#[test_log::test]
fn write() {
	let mut file = temp_file!("tests/files/assets/minimal/full_test.mka");

	let mut tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	assert_eq!(tagged_file.file_type(), FileType::Ebml);

	// Tags
	crate::set_artist!(tagged_file, tag_mut, TagType::Matroska, "Foo artist", 1 => file, "Bar artist");

	// Now reread the file
	file.rewind().unwrap();

	let mut tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	crate::set_artist!(tagged_file, tag_mut, TagType::Matroska, "Bar artist", 1 => file, "Foo artist");
}

#[test_log::test]
fn remove() {
	crate::remove_tag!(
		"tests/files/assets/minimal/full_test.mka",
		TagType::Matroska
	);
}

#[test_log::test]
fn read_no_properties() {
	crate::no_properties_test!("tests/files/assets/minimal/full_test.mka");
}

#[test_log::test]
fn read_no_tags() {
	crate::no_tag_test!("tests/files/assets/minimal/full_test.mka");
}
