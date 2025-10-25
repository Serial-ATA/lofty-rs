use crate::util::temp_file;
use lofty::config::{ParseOptions, WriteOptions};
use lofty::file::FileType;
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::TagType;

use std::io::Seek;

// The tests for OGG Opus/Vorbis/Speex are nearly identical
// We have the vendor string and a title stored in the tag

#[test_log::test]
fn opus_read() {
	read("tests/files/assets/minimal/full_test.opus", FileType::Opus)
}

#[test_log::test]
fn opus_write() {
	write("tests/files/assets/minimal/full_test.opus", FileType::Opus)
}

#[test_log::test]
fn opus_remove() {
	remove(
		"tests/files/assets/minimal/full_test.opus",
		TagType::VorbisComments,
	)
}

#[test_log::test]
fn flac_read() {
	// FLAC does **not** require a Vorbis comment block be present, this file has one
	read("tests/files/assets/minimal/full_test.flac", FileType::Flac)
}

#[test_log::test]
fn flac_write() {
	write("tests/files/assets/minimal/full_test.flac", FileType::Flac)
}

#[test_log::test]
fn flac_remove_vorbis_comments() {
	crate::util::remove_tag_test(
		"tests/files/assets/minimal/full_test.flac",
		TagType::VorbisComments,
	);
}

#[test_log::test]
fn vorbis_read() {
	read("tests/files/assets/minimal/full_test.ogg", FileType::Vorbis)
}

#[test_log::test]
fn vorbis_write() {
	write("tests/files/assets/minimal/full_test.ogg", FileType::Vorbis)
}

#[test_log::test]
fn vorbis_remove() {
	remove(
		"tests/files/assets/minimal/full_test.ogg",
		TagType::VorbisComments,
	)
}

#[test_log::test]
fn speex_read() {
	read("tests/files/assets/minimal/full_test.spx", FileType::Speex)
}

#[test_log::test]
fn speex_write() {
	write("tests/files/assets/minimal/full_test.spx", FileType::Speex)
}

#[test_log::test]
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

	// Expecting 2 items: vendor string and artist
	crate::util::verify_artist(&file, TagType::VorbisComments, "Foo artist", 2);
}

fn write(path: &str, file_type: FileType) {
	let mut tagged_file = crate::util::read(path);

	assert_eq!(tagged_file.file_type(), file_type);

	crate::util::set_artist(
		&mut tagged_file,
		TagType::VorbisComments,
		"Foo artist",
		"Bar artist",
		2,
	);

	// Now reread the file
	let mut file = tagged_file.into_inner();
	file.rewind().unwrap();

	let mut tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read_bound()
		.unwrap();

	crate::util::set_artist(
		&mut tagged_file,
		TagType::VorbisComments,
		"Bar artist",
		"Foo artist",
		2,
	);
}

fn remove(path: &str, tag_type: TagType) {
	let mut file = temp_file(path);

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

#[test_log::test]
fn flac_with_id3v2() {
	use lofty::flac::FlacFile;

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

// case TRACKNUMBER=11/22 (<current>/<total>)
#[test_log::test]
fn opus_issue_499() {
	use lofty::ogg::OpusFile;

	let file = std::fs::read("tests/files/assets/issue_499.opus").unwrap();
	let opus_file =
		OpusFile::read_from(&mut std::io::Cursor::new(file), ParseOptions::new()).unwrap();

	let comments = opus_file.vorbis_comments();
	assert_eq!(comments.track(), Some(11));
	assert_eq!(comments.track_total(), Some(22));
}

// case TRACKNUMBER=a5 (vinyl format)
#[test_log::test]
fn opus_issue_499_vinyl_track_number() {
	use lofty::ogg::OpusFile;

	let file = std::fs::read("tests/files/assets/issue_499_vinyl_track_number.opus").unwrap();
	let opus_file =
		OpusFile::read_from(&mut std::io::Cursor::new(file), ParseOptions::new()).unwrap();

	let comments = opus_file.vorbis_comments();
	assert_eq!(comments.track(), None);
	assert_eq!(comments.get("TRACKNUMBER"), Some("a5"));
}

#[test_log::test]
fn flac_remove_id3v2() {
	crate::util::remove_tag_test("tests/files/assets/flac_with_id3v2.flac", TagType::Id3v2);
}

#[test_log::test]
fn flac_try_write_non_empty_id3v2() {
	use lofty::id3::v2::Id3v2Tag;

	let mut tag = Id3v2Tag::default();
	tag.set_artist(String::from("Foo artist"));

	assert!(
		tag.save_to_path(
			"tests/files/assets/flac_with_id3v2.flac",
			WriteOptions::default()
		)
		.is_err()
	);
}

#[test_log::test]
fn read_no_properties_opus() {
	crate::util::no_properties_test("tests/files/assets/minimal/full_test.opus");
}

#[test_log::test]
fn read_no_tags_opus() {
	crate::util::no_tag_test("tests/files/assets/minimal/full_test.opus", Some(1));
}

#[test_log::test]
fn read_no_properties_vorbis() {
	crate::util::no_properties_test("tests/files/assets/minimal/full_test.ogg");
}

#[test_log::test]
fn read_no_tags_vorbis() {
	crate::util::no_tag_test("tests/files/assets/minimal/full_test.ogg", Some(1));
}

#[test_log::test]
fn read_no_properties_speex() {
	crate::util::no_properties_test("tests/files/assets/minimal/full_test.spx");
}

#[test_log::test]
fn read_no_tags_speex() {
	crate::util::no_tag_test("tests/files/assets/minimal/full_test.spx", Some(1));
}
