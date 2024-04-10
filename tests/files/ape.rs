use crate::{set_artist, temp_file, verify_artist};
use lofty::prelude::*;
use lofty::{FileType, ItemKey, ItemValue, ParseOptions, Probe, TagItem, TagType, TaggedFileExt};

use std::io::{Seek, Write};

#[test]
fn read() {
	// Here we have an APE file with an ID3v2, ID3v1, and an APEv2 tag
	let file = Probe::open("tests/files/assets/minimal/full_test.ape")
		.unwrap()
		.options(ParseOptions::new().read_properties(false))
		.read()
		.unwrap();

	assert_eq!(file.file_type(), FileType::Ape);

	// Verify the APEv2 tag first
	crate::verify_artist!(file, primary_tag, "Foo artist", 1);

	// Now verify ID3v1
	crate::verify_artist!(file, tag, TagType::Id3v1, "Bar artist", 1);

	// Finally, verify ID3v2
	crate::verify_artist!(file, tag, TagType::Id3v2, "Baz artist", 1);
}

#[test]
fn write() {
	// We don't write an ID3v2 tag here since it's against the spec
	let mut file = temp_file!("tests/files/assets/minimal/full_test.ape");

	let mut tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	assert_eq!(tagged_file.file_type(), FileType::Ape);

	// APEv2
	crate::set_artist!(tagged_file, primary_tag_mut, "Foo artist", 1 => file, "Bar artist");

	// ID3v1
	crate::set_artist!(tagged_file, tag_mut, TagType::Id3v1, "Bar artist", 1 => file, "Baz artist");

	// Now reread the file
	file.rewind().unwrap();
	let mut tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	crate::set_artist!(tagged_file, primary_tag_mut, "Bar artist", 1 => file, "Foo artist");

	crate::set_artist!(tagged_file, tag_mut, TagType::Id3v1, "Baz artist", 1 => file, "Bar artist");
}

#[test]
fn remove_ape() {
	crate::remove_tag!("tests/files/assets/minimal/full_test.ape", TagType::Ape);
}

#[test]
fn remove_id3v1() {
	crate::remove_tag!("tests/files/assets/minimal/full_test.ape", TagType::Id3v1);
}

#[test]
fn remove_id3v2() {
	crate::remove_tag!("tests/files/assets/minimal/full_test.ape", TagType::Id3v2);
}
