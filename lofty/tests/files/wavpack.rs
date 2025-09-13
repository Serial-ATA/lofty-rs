use crate::{set_artist, temp_file, verify_artist};
use lofty::ape::ApeTag;
use lofty::config::{ParseOptions, WriteOptions};
use lofty::file::FileType;
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::{Tag, TagType};

use std::io::Seek;

#[test_log::test]
fn read() {
	// Here we have a WacPack file with both an ID3v1 tag and an APE tag
	let file = Probe::open("tests/files/assets/minimal/full_test.wv")
		.unwrap()
		.options(ParseOptions::new().read_properties(false))
		.read()
		.unwrap();

	assert_eq!(file.file_type(), FileType::WavPack);

	// Verify the APE tag first
	crate::verify_artist!(file, primary_tag, "Foo artist", 1);

	// Now verify the ID3v1 tag
	crate::verify_artist!(file, tag, TagType::Id3v1, "Bar artist", 1);
}

#[test_log::test]
fn write() {
	let mut file = temp_file!("tests/files/assets/minimal/full_test.wv");

	let mut tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	assert_eq!(tagged_file.file_type(), FileType::WavPack);

	// APE
	set_artist!(tagged_file, primary_tag_mut, "Foo artist", 1 => file, "Bar artist");

	// ID3v1
	set_artist!(tagged_file, tag_mut, TagType::Id3v1, "Bar artist", 1 => file, "Baz artist");

	// Now reread the file
	file.rewind().unwrap();
	let mut tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	set_artist!(tagged_file, primary_tag_mut, "Bar artist", 1 => file, "Foo artist");

	set_artist!(tagged_file, tag_mut, TagType::Id3v1, "Baz artist", 1 => file, "Bar artist");
}

#[test_log::test]
fn remove_id3v1() {
	crate::remove_tag!("tests/files/assets/minimal/full_test.wv", TagType::Id3v1);
}

#[test_log::test]
fn remove_ape() {
	crate::remove_tag!("tests/files/assets/minimal/full_test.wv", TagType::Ape);
}

#[test_log::test]
fn read_no_properties() {
	crate::no_properties_test!("tests/files/assets/minimal/full_test.wv");
}

#[test_log::test]
fn read_no_tags() {
	crate::no_tag_test!("tests/files/assets/minimal/full_test.wv");
}

#[test_log::test]
fn write_ape_disc_key() {
	let mut file = crate::temp_file!("tests/files/assets/minimal/full_test.wv");
	let mut tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new())
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	// Create and insert a new Tag and set disk information
	let mut tag = Tag::new(TagType::Ape);
	tag.set_disk(3);
	tag.set_disk_total(5);
	tagged_file.insert_tag(tag);
	file.rewind().unwrap();
	tagged_file
		.save_to(&mut file, WriteOptions::default())
		.unwrap();

	// Reread the file to get the actual APE tag
	file.rewind().unwrap();
	let reread_tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new())
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();
	let tag_ref = reread_tagged_file.tag(TagType::Ape).unwrap();
	let ape_tag: ApeTag = tag_ref.clone().into();

	assert!(
		ape_tag.get("Disc").is_some(),
		"APE tag should contain `Disc` key with disk information"
	);
	assert_eq!(ape_tag.disk(), Some(3));
	assert_eq!(ape_tag.disk_total(), Some(5));
}
