use crate::util::temp_file;

use std::fs::File;
use std::io::Seek;

use lofty::config::{ParseOptions, ParsingMode, WriteOptions};
use lofty::flac::FlacFile;
use lofty::ogg::VorbisComments;
use lofty::prelude::*;

#[test_log::test]
fn multiple_vorbis_comments() {
	let mut file = File::open("tests/files/assets/two_vorbis_comments.flac").unwrap();

	// Reading a file with multiple VORBIS_COMMENT blocks should error when using `Strict`, as it is
	// not allowed by spec.
	assert!(FlacFile::read_from(
		&mut file,
		ParseOptions::new().parsing_mode(ParsingMode::Strict)
	)
	.is_err());

	file.rewind().unwrap();

	// But by default, we should just take the last tag in the stream
	let f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();

	// The first tag has the artist "Artist 1", the second has "Artist 2".
	assert_eq!(
		f.vorbis_comments().unwrap().artist().as_deref(),
		Some("Artist 2")
	);
}

#[test_log::test]
fn read_no_properties() {
	crate::util::no_properties_test("tests/files/assets/minimal/full_test.flac");
}

#[test_log::test]
fn read_no_tags() {
	crate::util::no_tag_test("tests/files/assets/minimal/full_test.flac", None);
}

#[test_log::test]
fn retain_vendor_string() {
	let mut file = temp_file("tests/files/assets/minimal/full_test.flac");

	let f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
	file.rewind().unwrap();

	assert_eq!(f.vorbis_comments().unwrap().vendor(), "Lavf58.76.100");

	let mut tag = VorbisComments::new();
	tag.set_artist(String::from("Foo Artist"));
	tag.set_vendor(String::from("Bar Vendor"));
	tag.save_to(&mut file, WriteOptions::new()).unwrap();

	file.rewind().unwrap();
	let f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();

	// The vendor string should be retained
	assert_eq!(f.vorbis_comments().unwrap().vendor(), "Lavf58.76.100");
}
