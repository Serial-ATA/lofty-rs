use crate::{set_artist, temp_file, verify_artist};
use lofty::{
	Accessor, FileType, ItemKey, ItemValue, ParseOptions, Probe, Tag, TagExt, TagItem, TagType,
	TaggedFileExt,
};
use std::io::{Seek, Write};

#[test]
fn read() {
	// Here we have an MP3 file with an ID3v2, ID3v1, and an APEv2 tag
	let file = Probe::open("tests/files/assets/minimal/full_test.mp3")
		.unwrap()
		.options(ParseOptions::new().read_properties(false))
		.read()
		.unwrap();

	assert_eq!(file.file_type(), FileType::MPEG);

	// Verify the ID3v2 tag first
	crate::verify_artist!(file, primary_tag, "Foo artist", 1);

	// Now verify ID3v1
	crate::verify_artist!(file, tag, TagType::ID3v1, "Bar artist", 1);

	// Finally, verify APEv2
	crate::verify_artist!(file, tag, TagType::APE, "Baz artist", 1);
}

#[test]
fn read_with_junk_bytes_between_frames() {
	// Read a file that includes an ID3v2.3 data block followed by four bytes of junk data (0x20)
	let file = Probe::open("tests/files/assets/junk_between_id3_and_mp3.mp3")
		.unwrap()
		.read()
		.unwrap();

	// note that the file contains ID3v2 and ID3v1 data
	assert_eq!(file.file_type(), FileType::MPEG);

	let id3v2_tag = &file.tags()[0];
	assert_eq!(id3v2_tag.artist().as_deref(), Some("artist test"));
	assert_eq!(id3v2_tag.album().as_deref(), Some("album test"));
	assert_eq!(id3v2_tag.title().as_deref(), Some("title test"));
	assert_eq!(
		id3v2_tag.get_string(&ItemKey::EncoderSettings),
		Some("Lavf58.62.100")
	);

	let id3v1_tag = &file.tags()[1];
	assert_eq!(id3v1_tag.artist().as_deref(), Some("artist test"));
	assert_eq!(id3v1_tag.album().as_deref(), Some("album test"));
	assert_eq!(id3v1_tag.title().as_deref(), Some("title test"));
}

#[test]
fn issue_82_solidus_in_tag() {
	let file = Probe::open("tests/files/assets/issue_82_solidus_in_tag.mp3")
		.unwrap()
		.read()
		.unwrap();

	assert_eq!(file.file_type(), FileType::MPEG);

	let id3v2_tag = &file.tags()[0];
	assert_eq!(id3v2_tag.title().as_deref(), Some("Foo / title"));
}

#[test]
fn issue_87_duplicate_id3v2() {
	// The first tag has a bunch of information: An album, artist, encoder, and a title.
	// This tag is immediately followed by another the contains an artist.
	// We expect that the title from the first tag has been replaced by this second tag, while
	// retaining the rest of the information from the first tag.
	let file = Probe::open("tests/files/assets/issue_87_duplicate_id3v2.mp3")
		.unwrap()
		.read()
		.unwrap();

	assert_eq!(file.file_type(), FileType::MPEG);

	let id3v2_tag = &file.tags()[0];
	assert_eq!(id3v2_tag.album().as_deref(), Some("album test"));
	assert_eq!(id3v2_tag.artist().as_deref(), Some("Foo artist")); // Original tag has "artist test"
	assert_eq!(
		id3v2_tag.get_string(&ItemKey::EncoderSettings),
		Some("Lavf58.62.100")
	);
	assert_eq!(id3v2_tag.title().as_deref(), Some("title test"));
}

#[test]
fn write() {
	let mut file = temp_file!("tests/files/assets/minimal/full_test.mp3");

	let mut tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	assert_eq!(tagged_file.file_type(), FileType::MPEG);

	// ID3v2
	crate::set_artist!(tagged_file, primary_tag_mut, "Foo artist", 1 => file, "Bar artist");

	// ID3v1
	crate::set_artist!(tagged_file, tag_mut, TagType::ID3v1, "Bar artist", 1 => file, "Baz artist");

	// APEv2
	crate::set_artist!(tagged_file, tag_mut, TagType::APE, "Baz artist", 1 => file, "Qux artist");

	// Now reread the file
	file.rewind().unwrap();
	let mut tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	crate::set_artist!(tagged_file, primary_tag_mut, "Bar artist", 1 => file, "Foo artist");

	crate::set_artist!(tagged_file, tag_mut, TagType::ID3v1, "Baz artist", 1 => file, "Bar artist");

	crate::set_artist!(tagged_file, tag_mut, TagType::APE, "Qux artist", 1 => file, "Baz artist");
}

#[test]
fn save_to_id3v2() {
	let mut file = temp_file!("tests/files/assets/minimal/full_test.mp3");

	let tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	assert_eq!(tagged_file.file_type(), FileType::MPEG);

	let mut tag = Tag::new(TagType::ID3v2);

	// Set title to save this tag.
	tag.set_title("title".to_string());

	file.rewind().unwrap();
	tag.save_to(&mut file).unwrap();

	// Now reread the file
	file.rewind().unwrap();
	let tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	let tag = tagged_file.tag(TagType::ID3v2).unwrap();

	assert!(tag.track().is_none());
	assert!(tag.track_total().is_none());
	assert!(tag.disk().is_none());
	assert!(tag.disk_total().is_none());
}

#[test]
fn save_number_of_track_and_disk_to_id3v2() {
	let mut file = temp_file!("tests/files/assets/minimal/full_test.mp3");

	let tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	assert_eq!(tagged_file.file_type(), FileType::MPEG);

	let mut tag = Tag::new(TagType::ID3v2);

	let track = 1;
	let disk = 2;

	tag.set_track(track);
	tag.set_disk(disk);

	file.rewind().unwrap();
	tag.save_to(&mut file).unwrap();

	// Now reread the file
	file.rewind().unwrap();
	let tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	let tag = tagged_file.tag(TagType::ID3v2).unwrap();

	assert_eq!(tag.track().unwrap(), track);
	assert!(tag.track_total().is_none());
	assert_eq!(tag.disk().unwrap(), disk);
	assert!(tag.disk_total().is_none());
}

#[test]
fn save_total_of_track_and_disk_to_id3v2() {
	let mut file = temp_file!("tests/files/assets/minimal/full_test.mp3");

	let tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	assert_eq!(tagged_file.file_type(), FileType::MPEG);

	let mut tag = Tag::new(TagType::ID3v2);

	let track_total = 2;
	let disk_total = 3;

	tag.set_track_total(track_total);
	tag.set_disk_total(disk_total);

	file.rewind().unwrap();
	tag.save_to(&mut file).unwrap();

	// Now reread the file
	file.rewind().unwrap();
	let tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	let tag = tagged_file.tag(TagType::ID3v2).unwrap();

	assert_eq!(tag.track().unwrap(), 0);
	assert_eq!(tag.track_total().unwrap(), track_total);
	assert_eq!(tag.disk().unwrap(), 0);
	assert_eq!(tag.disk_total().unwrap(), disk_total);
}

#[test]
fn save_number_pair_of_track_and_disk_to_id3v2() {
	let mut file = temp_file!("tests/files/assets/minimal/full_test.mp3");

	let tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	assert_eq!(tagged_file.file_type(), FileType::MPEG);

	let mut tag = Tag::new(TagType::ID3v2);

	let track = 1;
	let track_total = 2;
	let disk = 3;
	let disk_total = 4;

	tag.set_track(track);
	tag.set_track_total(track_total);

	tag.set_disk(disk);
	tag.set_disk_total(disk_total);

	file.rewind().unwrap();
	tag.save_to(&mut file).unwrap();

	// Now reread the file
	file.rewind().unwrap();
	let tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	let tag = tagged_file.tag(TagType::ID3v2).unwrap();

	assert_eq!(tag.track().unwrap(), track);
	assert_eq!(tag.track_total().unwrap(), track_total);
	assert_eq!(tag.disk().unwrap(), disk);
	assert_eq!(tag.disk_total().unwrap(), disk_total);
}

#[test]
fn remove_id3v2() {
	crate::remove_tag!("tests/files/assets/minimal/full_test.mp3", TagType::ID3v2);
}

#[test]
fn remove_id3v1() {
	crate::remove_tag!("tests/files/assets/minimal/full_test.mp3", TagType::ID3v1);
}

#[test]
fn remove_ape() {
	crate::remove_tag!("tests/files/assets/minimal/full_test.mp3", TagType::APE);
}
