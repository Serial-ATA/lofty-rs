use crate::{set_artist, temp_file, verify_artist};
use lofty::{
	FileType, ItemKey, ItemValue, ParseOptions, Probe, TagExt, TagItem, TagType, TaggedFileExt,
};
use std::io::{Seek, Write};

#[test]
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

#[test]
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

#[test]
fn remove_id3v1() {
	crate::remove_tag!("tests/files/assets/minimal/full_test.wv", TagType::Id3v1);
}

#[test]
fn remove_ape() {
	crate::remove_tag!("tests/files/assets/minimal/full_test.wv", TagType::Ape);
}
