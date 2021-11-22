mod util;

use lofty::{FileType, ItemKey, ItemValue, Probe, TagItem, TagType};
use std::io::{Seek, Write};

#[test]
fn read() {
	// Here we have a WAV file with both an ID3v2 chunk and a RIFF INFO chunk
	let file = Probe::new().read_from_path("tests/assets/a.wav").unwrap();

	assert_eq!(file.file_type(), &FileType::WAV);

	// Verify the ID3v2 tag first
	crate::verify_artist!(file, primary_tag, "Foo artist", 1);

	// Now verify the RIFF INFO chunk
	crate::verify_artist!(file, tag, TagType::RiffInfo, "Bar artist", 1);
}

#[test]
fn write() {
	let mut file = tempfile::tempfile().unwrap();
	file.write_all(&std::fs::read("tests/assets/a.wav").unwrap())
		.unwrap();

	let mut tagged_file = Probe::new().read_from(&mut file).unwrap();

	assert_eq!(tagged_file.file_type(), &FileType::WAV);

	// ID3v2
	crate::set_artist!(tagged_file, primary_tag_mut, "Foo artist", 1 => file, "Bar artist");

	// RIFF INFO
	crate::set_artist!(tagged_file, tag_mut, TagType::RiffInfo, "Bar artist", 1 => file, "Baz artist");

	drop(tagged_file);

	// Now reread the file
	let mut tagged_file = Probe::new().read_from(&mut file).unwrap();

	crate::set_artist!(tagged_file, primary_tag_mut, "Bar artist", 1 => file, "Foo artist");

	crate::set_artist!(tagged_file, tag_mut, TagType::RiffInfo, "Baz artist", 1 => file, "Bar artist");
}

#[test]
fn remove_id3v2() {
	crate::remove_tag!("tests/assets/a.wav", TagType::Id3v2);
}

#[test]
fn remove_riff_info() {
	crate::remove_tag!("tests/assets/a.wav", TagType::RiffInfo);
}
