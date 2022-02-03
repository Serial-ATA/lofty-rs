use crate::{set_artist, temp_file, verify_artist};
use lofty::{FileType, ItemKey, ItemValue, TagIO, TagItem, TagType};
use std::io::{Seek, SeekFrom, Write};

#[test]
fn read() {
	// This file contains an ilst atom
	let file = lofty::read_from_path("tests/files/assets/m4a_codec_aac.m4a", false).unwrap();

	assert_eq!(file.file_type(), &FileType::MP4);

	// Verify the ilst tag
	crate::verify_artist!(file, primary_tag, "Foo artist", 1);
}

#[test]
fn write() {
	let mut file = temp_file!("tests/files/assets/m4a_codec_aac.m4a");

	let mut tagged_file = lofty::read_from(&mut file, false).unwrap();

	assert_eq!(tagged_file.file_type(), &FileType::MP4);

	// ilst
	crate::set_artist!(tagged_file, tag_mut, TagType::Mp4Ilst, "Foo artist", 1 => file, "Bar artist");

	// Now reread the file
	file.seek(SeekFrom::Start(0)).unwrap();
	let mut tagged_file = lofty::read_from(&mut file, false).unwrap();

	crate::set_artist!(tagged_file, tag_mut, TagType::Mp4Ilst, "Bar artist", 1 => file, "Foo artist");
}

#[test]
fn remove() {
	crate::remove_tag!("tests/files/assets/m4a_codec_aac.m4a", TagType::Mp4Ilst);
}
