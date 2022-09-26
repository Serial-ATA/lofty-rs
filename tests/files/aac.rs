use crate::{set_artist, temp_file, verify_artist};
use lofty::{
	Accessor, FileType, ItemKey, ItemValue, ParseOptions, Probe, TagExt, TagItem, TagType,
};
use std::io::{Seek, Write};

// TODO

#[test]
#[ignore]
fn read() {
	// Here we have an AAC file with an ID3v2, ID3v1, and an APEv2 tag
	let file = Probe::open("tests/files/assets/minimal/full_test.aac")
		.unwrap()
		.options(ParseOptions::new().read_properties(false))
		.read()
		.unwrap();

	assert_eq!(file.file_type(), FileType::AAC);

	// Verify the ID3v2 tag first
	crate::verify_artist!(file, primary_tag, "Foo artist", 1);

	// Now verify ID3v1
	crate::verify_artist!(file, tag, TagType::ID3v1, "Bar artist", 1);

	// Finally, verify APEv2
	crate::verify_artist!(file, tag, TagType::APE, "Baz artist", 1);
}

#[test]
#[ignore]
fn read_with_junk_bytes_between_frames() {
	// Read a file that includes an ID3v2.3 data block followed by four bytes of junk data (0x20)

	// This is the same test as MP3, but it uses the same byte skipping logic, so it should be tested
	// here too :).
	let file = Probe::open("tests/files/assets/junk_between_id3_and_adts.aac")
		.unwrap()
		.read()
		.unwrap();

	// note that the file contains ID3v2 and ID3v1 data
	assert_eq!(file.file_type(), FileType::AAC);

	let id3v2_tag = &file.tags()[0];
	assert_eq!(id3v2_tag.artist(), Some("artist test"));
	assert_eq!(id3v2_tag.album(), Some("album test"));
	assert_eq!(id3v2_tag.title(), Some("title test"));
	assert_eq!(
		id3v2_tag.get_string(&ItemKey::EncoderSettings),
		Some("Lavf58.62.100")
	);

	let id3v1_tag = &file.tags()[1];
	assert_eq!(id3v1_tag.artist(), Some("artist test"));
	assert_eq!(id3v1_tag.album(), Some("album test"));
	assert_eq!(id3v1_tag.title(), Some("title test"));
}

#[test]
#[ignore]
fn write() {
	let mut file = temp_file!("tests/files/assets/minimal/full_test.aac");

	let mut tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	assert_eq!(tagged_file.file_type(), FileType::AAC);

	// ID3v2
	crate::set_artist!(tagged_file, primary_tag_mut, "Foo artist", 1 => file, "Bar artist");

	// ID3v1
	crate::set_artist!(tagged_file, tag_mut, TagType::ID3v1, "Bar artist", 1 => file, "Baz artist");

	// APEv2
	crate::set_artist!(tagged_file, tag_mut, TagType::APE, "Baz artist", 1 => file, "Qux artist");

	// Now reread the file
	file.rewind().unwrap();
	let mut tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	crate::set_artist!(tagged_file, primary_tag_mut, "Bar artist", 1 => file, "Foo artist");

	crate::set_artist!(tagged_file, tag_mut, TagType::ID3v1, "Baz artist", 1 => file, "Bar artist");

	crate::set_artist!(tagged_file, tag_mut, TagType::APE, "Qux artist", 1 => file, "Baz artist");
}

#[test]
#[ignore]
fn remove_id3v2() {
	crate::remove_tag!("tests/files/assets/minimal/full_test.aac", TagType::ID3v2);
}

#[test]
#[ignore]
fn remove_id3v1() {
	crate::remove_tag!("tests/files/assets/minimal/full_test.aac", TagType::ID3v1);
}

#[test]
#[ignore]
fn remove_ape() {
	crate::remove_tag!("tests/files/assets/minimal/full_test.aac", TagType::APE);
}
