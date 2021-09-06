mod util;

use lofty::{FileType, ItemKey, ItemValue, Probe, TagItem};

// The tests for OGG Opus/Vorbis are nearly identical
// We have the vendor string and a title stored in the tag

#[test]
fn ogg_opus_read() {
	read("tests/assets/a.opus", &FileType::Opus)
}

#[test]
fn ogg_opus_write() {
	write("tests/assets/a.opus", &FileType::Opus)
}

#[test]
fn ogg_flac_read() {
	// FLAC does **not** require a Vorbis comment block be present, this file has one
	read("tests/assets/a.flac", &FileType::FLAC)
}

#[test]
fn ogg_flac_write() {
	write("tests/assets/a.flac", &FileType::FLAC)
}

#[test]
fn ogg_vorbis_read() {
	read("tests/assets/a.ogg", &FileType::Vorbis)
}

#[test]
fn ogg_vorbis_write() {
	write("tests/assets/a.ogg", &FileType::Vorbis)
}

fn read(path: &str, file_type: &FileType) {
	let file = Probe::new().read_from_path(path).unwrap();

	assert_eq!(file.file_type(), file_type);

	crate::verify_artist!(file, primary_tag, "Foo artist", 2);
}

fn write(path: &str, file_type: &FileType) {
	let mut file = std::fs::OpenOptions::new()
		.read(true)
		.write(true)
		.open(path)
		.unwrap();

	let mut tagged_file = Probe::new().read_from(&mut file).unwrap();

	assert_eq!(tagged_file.file_type(), file_type);

	crate::set_artist!(tagged_file, primary_tag_mut, "Foo artist", 2 => file, "Bar artist");

	// Now reread the file
	let mut tagged_file = Probe::new().read_from(&mut file).unwrap();

	crate::set_artist!(tagged_file, primary_tag_mut, "Bar artist", 2 => file, "Foo artist");
}
