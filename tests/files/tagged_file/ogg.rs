use lofty::{FileType, ItemKey, ItemValue, Probe, TagItem, TagType};
use std::io::{Seek, Write};
use crate::{verify_artist, set_artist};

// The tests for OGG Opus/Vorbis are nearly identical
// We have the vendor string and a title stored in the tag

#[test]
fn opus_read() {
	read("tests/files/assets/a.opus", &FileType::Opus)
}

#[test]
fn opus_write() {
	write("tests/files/assets/a.opus", &FileType::Opus)
}

#[test]
fn opus_remove() {
	remove("tests/files/assets/a.opus", TagType::VorbisComments)
}

#[test]
fn flac_read() {
	// FLAC does **not** require a Vorbis comment block be present, this file has one
	read("tests/files/assets/a.flac", &FileType::FLAC)
}

#[test]
fn flac_write() {
	write("tests/files/assets/a.flac", &FileType::FLAC)
}

#[test]
fn flac_remove() {
	crate::remove_tag!("tests/files/assets/a.flac", TagType::VorbisComments);
}

#[test]
fn vorbis_read() {
	read("tests/files/assets/a.ogg", &FileType::Vorbis)
}

#[test]
fn vorbis_write() {
	write("tests/files/assets/a.ogg", &FileType::Vorbis)
}

#[test]
fn vorbis_remove() {
	remove("tests/files/assets/a.ogg", TagType::VorbisComments)
}

fn read(path: &str, file_type: &FileType) {
	let file = Probe::new().read_from_path(path).unwrap();

	assert_eq!(file.file_type(), file_type);

	crate::verify_artist!(file, primary_tag, "Foo artist", 2);
}

fn write(path: &str, file_type: &FileType) {
	let mut file = tempfile::tempfile().unwrap();
	file.write_all(&std::fs::read(path).unwrap()).unwrap();

	let mut tagged_file = Probe::new().read_from(&mut file).unwrap();

	assert_eq!(tagged_file.file_type(), file_type);

	crate::set_artist!(tagged_file, primary_tag_mut, "Foo artist", 2 => file, "Bar artist");

	drop(tagged_file);

	// Now reread the file
	let mut tagged_file = Probe::new().read_from(&mut file).unwrap();

	crate::set_artist!(tagged_file, primary_tag_mut, "Bar artist", 2 => file, "Foo artist");
}

fn remove(path: &str, tag_type: TagType) {
	let mut file = tempfile::tempfile().unwrap();
	file.write_all(&std::fs::read(path).unwrap()).unwrap();

	let tagged_file = Probe::new().read_from(&mut file).unwrap();
	// Verify we have both the vendor and artist
	assert!(
		tagged_file.tag(&tag_type).is_some()
			&& tagged_file.tag(&tag_type).unwrap().item_count() == 2
	);

	assert!(tag_type.remove_from(&mut file));

	let tagged_file = Probe::new().read_from(&mut file).unwrap();
	// We can't completely remove the tag since metadata packets are mandatory, but it should only have to vendor now
	assert_eq!(tagged_file.tag(&tag_type).unwrap().item_count(), 1);
}
