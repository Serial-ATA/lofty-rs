use crate::{set_artist, temp_file, verify_artist};
use lofty::{
	FileType, ItemKey, ItemValue, ParseOptions, Probe, TagExt, TagItem, TagType, TaggedFileExt,
};
use std::io::{Seek, Write};

#[test]
fn read() {
	// This file contains an ilst atom
	let file = Probe::open("tests/files/assets/minimal/m4a_codec_aac.m4a")
		.unwrap()
		.options(ParseOptions::new().read_properties(false))
		.read()
		.unwrap();

	assert_eq!(file.file_type(), FileType::MP4);

	// Verify the ilst tag
	crate::verify_artist!(file, primary_tag, "Foo artist", 1);
}

#[test]
fn write() {
	let mut file = temp_file!("tests/files/assets/minimal/m4a_codec_aac.m4a");

	let mut tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	assert_eq!(tagged_file.file_type(), FileType::MP4);

	// ilst
	crate::set_artist!(tagged_file, tag_mut, TagType::MP4ilst, "Foo artist", 1 => file, "Bar artist");

	// Now reread the file
	file.rewind().unwrap();

	let mut tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	crate::set_artist!(tagged_file, tag_mut, TagType::MP4ilst, "Bar artist", 1 => file, "Foo artist");
}

#[test]
fn remove() {
	crate::remove_tag!(
		"tests/files/assets/minimal/m4a_codec_aac.m4a",
		TagType::MP4ilst
	);
}
