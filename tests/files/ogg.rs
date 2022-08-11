use crate::{set_artist, temp_file, verify_artist};
use lofty::{FileType, ItemKey, ItemValue, TagExt, TagItem, TagType};
use std::io::{Seek, Write};

// The tests for OGG Opus/Vorbis are nearly identical
// We have the vendor string and a title stored in the tag

#[test]
fn opus_read() {
	read("tests/files/assets/minimal/full_test.opus", FileType::Opus)
}

#[test]
fn opus_write() {
	write("tests/files/assets/minimal/full_test.opus", FileType::Opus)
}

#[test]
fn opus_remove() {
	remove(
		"tests/files/assets/minimal/full_test.opus",
		TagType::VorbisComments,
	)
}

#[test]
fn flac_read() {
	// FLAC does **not** require a Vorbis comment block be present, this file has one
	read("tests/files/assets/minimal/full_test.flac", FileType::FLAC)
}

#[test]
fn flac_write() {
	write("tests/files/assets/minimal/full_test.flac", FileType::FLAC)
}

#[test]
fn flac_remove_vorbis_comments() {
	crate::remove_tag!(
		"tests/files/assets/minimal/full_test.flac",
		TagType::VorbisComments
	);
}

#[test]
fn vorbis_read() {
	read("tests/files/assets/minimal/full_test.ogg", FileType::Vorbis)
}

#[test]
fn vorbis_write() {
	write("tests/files/assets/minimal/full_test.ogg", FileType::Vorbis)
}

#[test]
fn vorbis_remove() {
	remove(
		"tests/files/assets/minimal/full_test.ogg",
		TagType::VorbisComments,
	)
}

#[test]
fn speex_read() {
	read("tests/files/assets/minimal/full_test.spx", FileType::Speex)
}

#[test]
fn speex_write() {
	write("tests/files/assets/minimal/full_test.spx", FileType::Speex)
}

#[test]
fn speex_remove() {
	remove(
		"tests/files/assets/minimal/full_test.spx",
		TagType::VorbisComments,
	)
}

fn read(path: &str, file_type: FileType) {
	let file = lofty::read_from_path(path, false).unwrap();

	assert_eq!(file.file_type(), file_type);

	crate::verify_artist!(file, primary_tag, "Foo artist", 2);
}

fn write(path: &str, file_type: FileType) {
	let mut file = temp_file!(path);

	let mut tagged_file = lofty::read_from(&mut file, false).unwrap();

	assert_eq!(tagged_file.file_type(), file_type);

	crate::set_artist!(tagged_file, primary_tag_mut, "Foo artist", 2 => file, "Bar artist");

	// Now reread the file
	file.rewind().unwrap();
	let mut tagged_file = lofty::read_from(&mut file, false).unwrap();

	crate::set_artist!(tagged_file, primary_tag_mut, "Bar artist", 2 => file, "Foo artist");
}

fn remove(path: &str, tag_type: TagType) {
	let mut file = temp_file!(path);

	let tagged_file = lofty::read_from(&mut file, false).unwrap();
	// Verify we have both the vendor and artist
	assert!(
		tagged_file.tag(tag_type).is_some() && tagged_file.tag(tag_type).unwrap().item_count() == 2
	);

	file.rewind().unwrap();
	tag_type.remove_from(&mut file).unwrap();

	file.rewind().unwrap();
	let tagged_file = lofty::read_from(&mut file, false).unwrap();

	// We can't completely remove the tag since metadata packets are mandatory, but it should only have to vendor now
	assert_eq!(tagged_file.tag(tag_type).unwrap().item_count(), 1);
}

#[test]
fn flac_with_id3v2() {
	use lofty::flac::FlacFile;
	use lofty::{Accessor, AudioFile};

	let file = std::fs::read("tests/files/assets/flac_with_id3v2.flac").unwrap();
	let flac_file = FlacFile::read_from(&mut std::io::Cursor::new(file), true).unwrap();

	assert!(flac_file.id3v2().is_some());
	assert_eq!(flac_file.id3v2().unwrap().artist(), Some("Foo artist"));

	assert!(flac_file.vorbis_comments().is_some());
}

#[test]
fn flac_remove_id3v2() {
	crate::remove_tag!("tests/files/assets/flac_with_id3v2.flac", TagType::ID3v2);
}
