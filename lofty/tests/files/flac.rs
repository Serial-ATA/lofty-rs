use crate::util::temp_file;

use std::fs::File;
use std::io::Seek;

use lofty::config::{ParseOptions, ParsingMode, WriteOptions};
use lofty::flac::FlacFile;
use lofty::ogg::{OggPictureStorage, VorbisComments};
use lofty::picture::{Picture, PictureInformation, PictureType};
use lofty::prelude::*;

#[test_log::test]
fn multiple_vorbis_comments() {
	let mut file = File::open("tests/files/assets/two_vorbis_comments.flac").unwrap();

	// Reading a file with multiple VORBIS_COMMENT blocks should error when using `Strict`, as it is
	// not allowed by spec.
	assert!(
		FlacFile::read_from(
			&mut file,
			ParseOptions::new()
				.read_properties(false)
				.parsing_mode(ParsingMode::Strict)
		)
		.is_err()
	);

	file.rewind().unwrap();

	// But by default, we should just take the last tag in the stream
	let f = FlacFile::read_from(&mut file, ParseOptions::new().read_properties(false)).unwrap();

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

// The final written metadata block will be vorbis comments
#[test_log::test]
fn stream_info_is_last() {
	// The file only has a stream info metadata block
	let mut file = temp_file("tests/files/assets/stream_info_last.flac");

	let f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
	file.rewind().unwrap();

	assert!(f.vorbis_comments().is_none());

	let mut tag = VorbisComments::new();
	tag.set_artist(String::from("Foo Artist"));
	tag.save_to(&mut file, WriteOptions::new()).unwrap();

	file.rewind().unwrap();
	let f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();

	assert_eq!(
		f.vorbis_comments().unwrap().artist().as_deref(),
		Some("Foo Artist")
	);
}

// Same as previous test, except the new tag will be written with no padding
#[test_log::test]
fn stream_info_is_last_no_padding() {
	let mut file = temp_file("tests/files/assets/stream_info_last.flac");

	let f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
	file.rewind().unwrap();

	assert!(f.vorbis_comments().is_none());

	let mut tag = VorbisComments::new();
	tag.set_artist(String::from("Foo Artist"));
	tag.save_to(&mut file, WriteOptions::new().preferred_padding(0))
		.unwrap();

	file.rewind().unwrap();
	let f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();

	assert_eq!(
		f.vorbis_comments().unwrap().artist().as_deref(),
		Some("Foo Artist")
	);

	// Stripping the Vorbis Comments should set the `STREAMINFO` block as the last metadata block
	file.rewind().unwrap();
	let tag = VorbisComments::new();
	tag.save_to(&mut file, WriteOptions::new().preferred_padding(0))
		.unwrap();

	file.rewind().unwrap();
	let f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();

	assert!(f.vorbis_comments().is_none());
}

// The final written metadata block will be a picture
#[test_log::test]
fn picture_is_last() {
	let mut file = temp_file("tests/files/assets/stream_info_last.flac");

	let mut f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
	file.rewind().unwrap();

	assert!(f.vorbis_comments().is_none());
	assert!(f.pictures().is_empty());

	f.insert_picture(
		Picture::unchecked(Vec::new())
			.pic_type(PictureType::CoverFront)
			.build(),
		Some(PictureInformation {
			width: 200,
			height: 200,
			color_depth: 0,
			num_colors: 16,
		}),
	)
	.unwrap();
	f.save_to(&mut file, WriteOptions::new().preferred_padding(0))
		.unwrap();

	file.rewind().unwrap();
	let f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();

	assert_eq!(f.pictures().len(), 1);
}

#[test_log::test]
fn application_block_is_last() {
	let mut file = temp_file("tests/files/assets/application_block_last.flac");

	let f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
	file.rewind().unwrap();

	assert!(f.vorbis_comments().is_none());

	let mut tag = VorbisComments::new();
	tag.set_artist(String::from("Foo Artist"));
	tag.save_to(&mut file, WriteOptions::new().preferred_padding(0))
		.unwrap();

	file.rewind().unwrap();
	let f = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();

	assert_eq!(
		f.vorbis_comments().unwrap().artist().as_deref(),
		Some("Foo Artist")
	);
}
