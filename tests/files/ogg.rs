use crate::{set_artist, temp_file, verify_artist};
use lofty::{
	FileType, ItemKey, ItemValue, ParseOptions, Probe, TagExt, TagItem, TagType, TaggedFileExt,
	WriteOptions,
};
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
	read("tests/files/assets/minimal/full_test.flac", FileType::Flac)
}

#[test]
fn flac_write() {
	write("tests/files/assets/minimal/full_test.flac", FileType::Flac)
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
	let file = Probe::open(path)
		.unwrap()
		.options(ParseOptions::new().read_properties(false))
		.read()
		.unwrap();

	assert_eq!(file.file_type(), file_type);

	crate::verify_artist!(file, primary_tag, "Foo artist", 2);
}

fn write(path: &str, file_type: FileType) {
	let mut file = temp_file!(path);

	let mut tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	assert_eq!(tagged_file.file_type(), file_type);

	crate::set_artist!(tagged_file, primary_tag_mut, "Foo artist", 2 => file, "Bar artist");

	// Now reread the file
	file.rewind().unwrap();
	let mut tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	crate::set_artist!(tagged_file, primary_tag_mut, "Bar artist", 2 => file, "Foo artist");
}

fn remove(path: &str, tag_type: TagType) {
	let mut file = temp_file!(path);

	let tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();
	// Verify we have both the vendor and artist
	assert!(
		tagged_file.tag(tag_type).is_some() && tagged_file.tag(tag_type).unwrap().item_count() == 2
	);

	file.rewind().unwrap();
	tag_type.remove_from(&mut file).unwrap();

	file.rewind().unwrap();
	let tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	// We can't completely remove the tag since metadata packets are mandatory, but it should only have to vendor now
	assert_eq!(tagged_file.tag(tag_type).unwrap().item_count(), 1);
}

#[test]
fn flac_with_id3v2() {
	use lofty::flac::FlacFile;
	use lofty::{Accessor, AudioFile};

	let file = std::fs::read("tests/files/assets/flac_with_id3v2.flac").unwrap();
	let flac_file =
		FlacFile::read_from(&mut std::io::Cursor::new(file), ParseOptions::new()).unwrap();

	assert!(flac_file.id3v2().is_some());
	assert_eq!(
		flac_file.id3v2().unwrap().artist().as_deref(),
		Some("Foo artist")
	);

	assert!(flac_file.vorbis_comments().is_some());
}

#[test]
fn flac_remove_id3v2() {
	crate::remove_tag!("tests/files/assets/flac_with_id3v2.flac", TagType::Id3v2);
}

#[test]
fn flac_try_write_non_empty_id3v2() {
	use lofty::id3::v2::Id3v2Tag;
	use lofty::Accessor;

	let mut tag = Id3v2Tag::default();
	tag.set_artist(String::from("Foo artist"));

	assert!(tag
		.save_to_path(
			"tests/files/assets/flac_with_id3v2.flac",
			WriteOptions::default()
		)
		.is_err());
}
