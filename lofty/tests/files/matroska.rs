use crate::{set_artist, temp_file, verify_artist};
use lofty::config::ParseOptions;
use lofty::ebml::EbmlFile;
use lofty::file::FileType;
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::TagType;

use std::fs::File;
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

// Official Matroska test files
#[test_log::test]
fn basic_file() {
	let mut f = File::open("tests/files/assets/matroska-test-files/test1.mkv").unwrap();
	let _ = EbmlFile::read_from(&mut f, ParseOptions::default()).unwrap();
}

#[test_log::test]
fn non_default_timecodescale() {
	let mut f = File::open("tests/files/assets/matroska-test-files/test2.mkv").unwrap();
	let _ = EbmlFile::read_from(&mut f, ParseOptions::default()).unwrap();
}

#[test_log::test]
fn header_stripping_standard_block() {
	let mut f = File::open("tests/files/assets/matroska-test-files/test3.mkv").unwrap();
	let _ = EbmlFile::read_from(&mut f, ParseOptions::default()).unwrap();
}

#[test_log::test]
fn live_stream() {
	let mut f = File::open("tests/files/assets/matroska-test-files/test4.mkv").unwrap();
	let _ = EbmlFile::read_from(&mut f, ParseOptions::default()).unwrap();
}

#[test_log::test]
fn multiple_audio_subtitles() {
	let mut f = File::open("tests/files/assets/matroska-test-files/test5.mkv").unwrap();
	let _ = EbmlFile::read_from(&mut f, ParseOptions::default()).unwrap();
}

#[test_log::test]
fn different_header_sizes_cueless_seeking() {
	let mut f = File::open("tests/files/assets/matroska-test-files/test6.mkv").unwrap();
	let _ = EbmlFile::read_from(&mut f, ParseOptions::default()).unwrap();
}

#[test_log::test]
fn extra_unknown_junk_elements_damaged() {
	let mut f = File::open("tests/files/assets/matroska-test-files/test7.mkv").unwrap();
	let _ = EbmlFile::read_from(&mut f, ParseOptions::default()).unwrap();
}

#[test_log::test]
fn audio_gap() {
	let mut f = File::open("tests/files/assets/matroska-test-files/test8.mkv").unwrap();
	let _ = EbmlFile::read_from(&mut f, ParseOptions::default()).unwrap();
}
