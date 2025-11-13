use crate::util::temp_file;
use lofty::config::{ParseOptions, WriteOptions};
use lofty::file::{BoundTaggedFile, FileType};
use lofty::id3::v2::{Frame, FrameId, Id3v2Tag, KeyValueFrame};
use lofty::mpeg::MpegFile;
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::{Tag, TagType};

use std::borrow::Cow;
use std::io::Seek;

#[test_log::test]
fn read() {
	// Here we have an MP3 file with an ID3v2, ID3v1, and an APEv2 tag
	let file = Probe::open("tests/files/assets/minimal/full_test.mp3")
		.unwrap()
		.options(ParseOptions::new().read_properties(false))
		.read()
		.unwrap();

	assert_eq!(file.file_type(), FileType::Mpeg);

	// Verify the ID3v2 tag first
	crate::util::verify_artist(&file, TagType::Id3v2, "Foo artist", 1);

	// Now verify ID3v1
	crate::util::verify_artist(&file, TagType::Id3v1, "Bar artist", 1);

	// Finally, verify APEv2
	crate::util::verify_artist(&file, TagType::Ape, "Baz artist", 1);
}

#[test_log::test]
fn read_with_junk_bytes_between_frames() {
	// Read a file that includes an ID3v2.3 data block followed by four bytes of junk data (0x20)
	let file = Probe::open("tests/files/assets/junk_between_id3_and_mp3.mp3")
		.unwrap()
		.read()
		.unwrap();

	// note that the file contains ID3v2 and ID3v1 data
	assert_eq!(file.file_type(), FileType::Mpeg);

	let id3v2_tag = &file.tags()[0];
	assert_eq!(id3v2_tag.artist().as_deref(), Some("artist test"));
	assert_eq!(id3v2_tag.album().as_deref(), Some("album test"));
	assert_eq!(id3v2_tag.title().as_deref(), Some("title test"));
	assert_eq!(
		id3v2_tag.get_string(ItemKey::EncoderSettings),
		Some("Lavf58.62.100")
	);

	let id3v1_tag = &file.tags()[1];
	assert_eq!(id3v1_tag.artist().as_deref(), Some("artist test"));
	assert_eq!(id3v1_tag.album().as_deref(), Some("album test"));
	assert_eq!(id3v1_tag.title().as_deref(), Some("title test"));
}

#[test_log::test]
fn issue_82_solidus_in_tag() {
	let file = Probe::open("tests/files/assets/issue_82_solidus_in_tag.mp3")
		.unwrap()
		.read()
		.unwrap();

	assert_eq!(file.file_type(), FileType::Mpeg);

	let id3v2_tag = &file.tags()[0];
	assert_eq!(id3v2_tag.title().as_deref(), Some("Foo / title"));
}

#[test_log::test]
fn issue_87_duplicate_id3v2() {
	// The first tag has a bunch of information: An album, artist, encoder, and a title.
	// This tag is immediately followed by another the contains an artist.
	// We expect that the title from the first tag has been replaced by this second tag, while
	// retaining the rest of the information from the first tag.
	let file = Probe::open("tests/files/assets/issue_87_duplicate_id3v2.mp3")
		.unwrap()
		.read()
		.unwrap();

	assert_eq!(file.file_type(), FileType::Mpeg);

	let id3v2_tag = &file.tags()[0];
	assert_eq!(id3v2_tag.album().as_deref(), Some("album test"));
	assert_eq!(id3v2_tag.artist().as_deref(), Some("Foo artist")); // Original tag has "artist test"
	assert_eq!(
		id3v2_tag.get_string(ItemKey::EncoderSettings),
		Some("Lavf58.62.100")
	);
	assert_eq!(id3v2_tag.title().as_deref(), Some("title test"));
}

#[test_log::test]
fn write() {
	let mut tagged_file = crate::util::read("tests/files/assets/minimal/full_test.mp3");

	assert_eq!(tagged_file.file_type(), FileType::Mpeg);

	// ID3v2
	crate::util::set_artist(
		&mut tagged_file,
		TagType::Id3v2,
		"Foo artist",
		"Bar artist",
		1,
	);

	// ID3v1
	crate::util::set_artist(
		&mut tagged_file,
		TagType::Id3v1,
		"Bar artist",
		"Baz artist",
		1,
	);

	// APEv2
	crate::util::set_artist(
		&mut tagged_file,
		TagType::Ape,
		"Baz artist",
		"Qux artist",
		1,
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
		TagType::Id3v2,
		"Bar artist",
		"Foo artist",
		1,
	);

	crate::util::set_artist(
		&mut tagged_file,
		TagType::Id3v1,
		"Baz artist",
		"Bar artist",
		1,
	);

	crate::util::set_artist(
		&mut tagged_file,
		TagType::Ape,
		"Qux artist",
		"Baz artist",
		1,
	);
}

#[test_log::test]
fn save_to_id3v2() {
	let mut file = temp_file("tests/files/assets/minimal/full_test.mp3");

	let tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	assert_eq!(tagged_file.file_type(), FileType::Mpeg);

	let mut tag = Tag::new(TagType::Id3v2);

	// Set title to save this tag.
	tag.set_title("title".to_string());

	file.rewind().unwrap();
	tag.save_to(&mut file, WriteOptions::default()).unwrap();

	// Now reread the file
	file.rewind().unwrap();
	let tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	let tag = tagged_file.tag(TagType::Id3v2).unwrap();

	assert!(tag.track().is_none());
	assert!(tag.track_total().is_none());
	assert!(tag.disk().is_none());
	assert!(tag.disk_total().is_none());
}

#[test_log::test]
fn save_number_of_track_and_disk_to_id3v2() {
	let mut file = temp_file("tests/files/assets/minimal/full_test.mp3");

	let tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	assert_eq!(tagged_file.file_type(), FileType::Mpeg);

	let mut tag = Tag::new(TagType::Id3v2);

	let track = 1;
	let disk = 2;

	tag.set_track(track);
	tag.set_disk(disk);

	file.rewind().unwrap();
	tag.save_to(&mut file, WriteOptions::default()).unwrap();

	// Now reread the file
	file.rewind().unwrap();
	let tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	let tag = tagged_file.tag(TagType::Id3v2).unwrap();

	assert_eq!(tag.track().unwrap(), track);
	assert!(tag.track_total().is_none());
	assert_eq!(tag.disk().unwrap(), disk);
	assert!(tag.disk_total().is_none());
}

#[test_log::test]
fn test_bound_tagged_into_inner() {
	let file = temp_file("tests/files/assets/minimal/full_test.mp3");

	let mut bounded = BoundTaggedFile::read_from(file, ParseOptions::default()).unwrap();

	let tag = bounded
		.tag_mut(TagType::Id3v2)
		.expect("Couldn't get ref to tag");
	tag.set_disk(123);
	bounded
		.save(WriteOptions::default())
		.expect("Couldn't save tags");

	// Reread the file
	let mut original_file = bounded.into_inner();
	original_file.rewind().unwrap();
	let mut bounded = BoundTaggedFile::read_from(original_file, ParseOptions::default()).unwrap();
	let tag = bounded.tag_mut(TagType::Id3v2).unwrap();

	assert_eq!(tag.disk(), Some(123));
}

#[test_log::test]
fn save_total_of_track_and_disk_to_id3v2() {
	let mut file = temp_file("tests/files/assets/minimal/full_test.mp3");

	let tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	assert_eq!(tagged_file.file_type(), FileType::Mpeg);

	let mut tag = Tag::new(TagType::Id3v2);

	let track_total = 2;
	let disk_total = 3;

	tag.set_track_total(track_total);
	tag.set_disk_total(disk_total);

	file.rewind().unwrap();
	tag.save_to(&mut file, WriteOptions::default()).unwrap();

	// Now reread the file
	file.rewind().unwrap();
	let tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	let tag = tagged_file.tag(TagType::Id3v2).unwrap();

	assert_eq!(tag.track().unwrap(), 0);
	assert_eq!(tag.track_total().unwrap(), track_total);
	assert_eq!(tag.disk().unwrap(), 0);
	assert_eq!(tag.disk_total().unwrap(), disk_total);
}

#[test_log::test]
fn save_number_pair_of_track_and_disk_to_id3v2() {
	let mut file = temp_file("tests/files/assets/minimal/full_test.mp3");

	let tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	assert_eq!(tagged_file.file_type(), FileType::Mpeg);

	let mut tag = Tag::new(TagType::Id3v2);

	let track = 1;
	let track_total = 2;
	let disk = 3;
	let disk_total = 4;

	tag.set_track(track);
	tag.set_track_total(track_total);

	tag.set_disk(disk);
	tag.set_disk_total(disk_total);

	file.rewind().unwrap();
	tag.save_to(&mut file, WriteOptions::default()).unwrap();

	// Now reread the file
	file.rewind().unwrap();
	let tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	let tag = tagged_file.tag(TagType::Id3v2).unwrap();

	assert_eq!(tag.track().unwrap(), track);
	assert_eq!(tag.track_total().unwrap(), track_total);
	assert_eq!(tag.disk().unwrap(), disk);
	assert_eq!(tag.disk_total().unwrap(), disk_total);
}

#[test_log::test]
fn remove_id3v2() {
	crate::util::remove_tag_test("tests/files/assets/minimal/full_test.mp3", TagType::Id3v2);
}

#[test_log::test]
fn remove_id3v1() {
	crate::util::remove_tag_test("tests/files/assets/minimal/full_test.mp3", TagType::Id3v1);
}

#[test_log::test]
fn remove_ape() {
	crate::util::remove_tag_test("tests/files/assets/minimal/full_test.mp3", TagType::Ape);
}

#[test_log::test]
fn read_and_write_tpil_frame() {
	let key_value_pairs = vec![
		(Cow::Borrowed("engineer"), Cow::Borrowed("testperson")),
		(Cow::Borrowed("vocalist"), Cow::Borrowed("testhuman")),
	];

	let mut file = temp_file("tests/files/assets/minimal/full_test.mp3");

	let mut mpeg_file = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();

	let tag: &mut Id3v2Tag = mpeg_file.id3v2_mut().unwrap();

	tag.insert(Frame::KeyValue(KeyValueFrame::new(
		FrameId::Valid(Cow::Borrowed("TIPL")),
		lofty::TextEncoding::UTF8,
		key_value_pairs.clone(),
	)));

	file.rewind().unwrap();
	tag.save_to(&mut file, WriteOptions::default()).unwrap();

	// Now reread the file
	file.rewind().unwrap();
	let mpeg_file = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();

	let tag: &Id3v2Tag = mpeg_file.id3v2().unwrap();

	let Frame::KeyValue(content) = tag.get(&FrameId::Valid(Cow::Borrowed("TIPL"))).unwrap() else {
		panic!("Wrong Frame Value Type for TIPL")
	};

	assert_eq!(key_value_pairs, content.key_value_pairs);
}

#[test_log::test]
fn read_no_properties() {
	let mut file = temp_file("tests/files/assets/minimal/full_test.mp3");
	let tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();
	let properties = tagged_file.properties();
	assert!(properties.duration().is_zero());
	assert_eq!(properties.overall_bitrate(), Some(0));
	assert_eq!(properties.audio_bitrate(), Some(0));
	assert_eq!(properties.sample_rate(), Some(0));
	assert!(properties.bit_depth().is_none());
	assert_eq!(properties.channels(), Some(0));
}

#[test_log::test]
fn read_no_tags() {
	crate::util::no_tag_test("tests/files/assets/minimal/full_test.mp3", None);
}
