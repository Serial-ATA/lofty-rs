use lofty::{FileType, ItemKey, ItemValue, Probe, TagItem, TagType};
use std::io::{Seek, Write};
use crate::set_artist;
use crate::verify_artist;

#[test]
fn read() {
	// Here we have an AIFF file with both an ID3v2 chunk and text chunks
	let file = Probe::new().read_from_path("tests/files/assets/a.aiff").unwrap();

	assert_eq!(file.file_type(), &FileType::AIFF);

	// Verify the ID3v2 tag first
	crate::verify_artist!(file, primary_tag, "Foo artist", 1);

	// Now verify the text chunks
	crate::verify_artist!(file, tag, TagType::AiffText, "Bar artist", 1);
}

#[test]
fn write() {
	let mut file = tempfile::tempfile().unwrap();
	file.write_all(&std::fs::read("tests/files/assets/a.aiff").unwrap())
		.unwrap();

	let mut tagged_file = Probe::new().read_from(&mut file).unwrap();

	assert_eq!(tagged_file.file_type(), &FileType::AIFF);

	// ID3v2
	crate::set_artist!(tagged_file, primary_tag_mut, "Foo artist", 1 => file, "Bar artist");

	// Text chunks
	crate::set_artist!(tagged_file, tag_mut, TagType::AiffText, "Bar artist", 1 => file, "Baz artist");

	drop(tagged_file);

	// Now reread the file
	let mut tagged_file = Probe::new().read_from(&mut file).unwrap();

	crate::set_artist!(tagged_file, primary_tag_mut, "Bar artist", 1 => file, "Foo artist");

	crate::set_artist!(tagged_file, tag_mut, TagType::AiffText, "Baz artist", 1 => file, "Bar artist");
}

#[test]
fn remove_text_chunks() {
	crate::remove_tag!("tests/files/assets/a.aiff", TagType::AiffText);
}

#[test]
fn remove_id3v2() {
	crate::remove_tag!("tests/files/assets/a.aiff", TagType::Id3v2);
}
