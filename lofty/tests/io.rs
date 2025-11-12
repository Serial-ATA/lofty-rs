#![allow(missing_docs)]

use std::io::{Cursor, Read, Seek, Write};

use lofty::config::{ParseOptions, WriteOptions};
use lofty::file::AudioFile;
use lofty::mpeg::MpegFile;
use lofty::tag::Accessor;

const TEST_ASSET: &str = "tests/files/assets/minimal/full_test.mp3";

fn test_asset_contents() -> Vec<u8> {
	std::fs::read(TEST_ASSET).unwrap()
}

fn file() -> MpegFile {
	let file_contents = test_asset_contents();
	let mut reader = Cursor::new(file_contents);
	MpegFile::read_from(&mut reader, ParseOptions::new()).unwrap()
}

fn alter_tag(file: &mut MpegFile) {
	let tag = file.id3v2_mut().unwrap();
	tag.set_artist(String::from("Bar artist"));
}

fn revert_tag(file: &mut MpegFile) {
	let tag = file.id3v2_mut().unwrap();
	tag.set_artist(String::from("Foo artist"));
}

#[test_log::test]
fn io_save_to_file() {
	// Read the file and change the artist
	let mut file = file();
	alter_tag(&mut file);

	let mut temp_file = tempfile::tempfile().unwrap();
	let file_content = std::fs::read(TEST_ASSET).unwrap();
	temp_file.write_all(&file_content).unwrap();
	temp_file.rewind().unwrap();

	// Save the new artist
	file.save_to(&mut temp_file, WriteOptions::new().preferred_padding(0))
		.expect("Failed to save to file");

	// Read the file again and change the artist back
	temp_file.rewind().unwrap();
	let mut file = MpegFile::read_from(&mut temp_file, ParseOptions::new()).unwrap();
	revert_tag(&mut file);

	temp_file.rewind().unwrap();
	file.save_to(&mut temp_file, WriteOptions::new().preferred_padding(0))
		.expect("Failed to save to file");

	// The contents should be the same as the original file
	temp_file.rewind().unwrap();
	let mut current_file_contents = Vec::new();
	temp_file.read_to_end(&mut current_file_contents).unwrap();

	assert_eq!(current_file_contents, test_asset_contents());
}

#[test_log::test]
fn io_save_to_vec() {
	// Same test as above, but using a Cursor<Vec<u8>> instead of a file
	let mut file = file();
	alter_tag(&mut file);

	let file_content = std::fs::read(TEST_ASSET).unwrap();

	let mut reader = Cursor::new(file_content);
	file.save_to(&mut reader, WriteOptions::new().preferred_padding(0))
		.expect("Failed to save to vec");

	reader.rewind().unwrap();
	let mut file = MpegFile::read_from(&mut reader, ParseOptions::new()).unwrap();
	revert_tag(&mut file);

	reader.rewind().unwrap();
	file.save_to(&mut reader, WriteOptions::new().preferred_padding(0))
		.expect("Failed to save to vec");

	let current_file_contents = reader.into_inner();
	assert_eq!(current_file_contents, test_asset_contents());
}

#[test_log::test]
fn io_save_using_references() {
	struct File {
		buf: Vec<u8>,
	}

	let mut f = File {
		buf: std::fs::read(TEST_ASSET).unwrap(),
	};

	// Same test as above, but using references instead of owned values
	let mut file = file();
	alter_tag(&mut file);

	{
		let mut reader = Cursor::new(&mut f.buf);
		file.save_to(&mut reader, WriteOptions::new().preferred_padding(0))
			.expect("Failed to save to vec");
	}

	{
		let mut reader = Cursor::new(&f.buf[..]);
		file = MpegFile::read_from(&mut reader, ParseOptions::new()).unwrap();
		revert_tag(&mut file);
	}

	{
		let mut reader = Cursor::new(&mut f.buf);
		file.save_to(&mut reader, WriteOptions::new().preferred_padding(0))
			.expect("Failed to save to vec");
	}

	let current_file_contents = f.buf;
	assert_eq!(current_file_contents, test_asset_contents());
}
