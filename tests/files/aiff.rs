use crate::{set_artist, temp_file, verify_artist};
use lofty::{
	FileType, ItemKey, ItemValue, ParseOptions, Probe, TagExt, TagItem, TagType, TaggedFileExt,
};
use std::io::{Seek, Write};

#[test]
fn read() {
	// Here we have an AIFF file with both an ID3v2 chunk and text chunks
	let file = Probe::open("tests/files/assets/minimal/full_test.aiff")
		.unwrap()
		.options(ParseOptions::new().read_properties(false))
		.read()
		.unwrap();

	assert_eq!(file.file_type(), FileType::AIFF);

	// Verify the ID3v2 tag first
	crate::verify_artist!(file, primary_tag, "Foo artist", 1);

	// Now verify the text chunks
	crate::verify_artist!(file, tag, TagType::AIFFText, "Bar artist", 1);
}

#[test]
fn write() {
	let mut file = temp_file!("tests/files/assets/minimal/full_test.aiff");

	let mut tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	assert_eq!(tagged_file.file_type(), FileType::AIFF);

	// ID3v2
	crate::set_artist!(tagged_file, primary_tag_mut, "Foo artist", 1 => file, "Bar artist");

	// Text chunks
	crate::set_artist!(tagged_file, tag_mut, TagType::AIFFText, "Bar artist", 1 => file, "Baz artist");

	// Now reread the file
	file.rewind().unwrap();
	let mut tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	crate::set_artist!(tagged_file, primary_tag_mut, "Bar artist", 1 => file, "Foo artist");

	crate::set_artist!(tagged_file, tag_mut, TagType::AIFFText, "Baz artist", 1 => file, "Bar artist");
}

#[test]
fn remove_text_chunks() {
	crate::remove_tag!(
		"tests/files/assets/minimal/full_test.aiff",
		TagType::AIFFText
	);
}

#[test]
fn remove_id3v2() {
	crate::remove_tag!("tests/files/assets/minimal/full_test.aiff", TagType::ID3v2);
}
