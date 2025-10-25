use lofty::config::ParseOptions;
use lofty::file::FileType;
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::TagType;

use std::io::Seek;

#[test_log::test]
fn read() {
	// Here we have a WAV file with both an ID3v2 chunk and a RIFF INFO chunk
	let file = Probe::open("tests/files/assets/minimal/wav_format_pcm.wav")
		.unwrap()
		.options(ParseOptions::new().read_properties(false))
		.read()
		.unwrap();

	assert_eq!(file.file_type(), FileType::Wav);

	// Verify the ID3v2 tag first
	crate::util::verify_artist(&file, TagType::Id3v2, "Foo artist", 1);

	// Now verify the RIFF INFO chunk
	crate::util::verify_artist(&file, TagType::RiffInfo, "Bar artist", 1);
}

#[test_log::test]
fn write() {
	let mut tagged_file = crate::util::read("tests/files/assets/minimal/wav_format_pcm.wav");

	assert_eq!(tagged_file.file_type(), FileType::Wav);

	// ID3v2
	crate::util::set_artist(
		&mut tagged_file,
		TagType::Id3v2,
		"Foo artist",
		"Bar artist",
		1,
	);

	// RIFF INFO
	crate::util::set_artist(
		&mut tagged_file,
		TagType::RiffInfo,
		"Bar artist",
		"Baz artist",
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
		TagType::RiffInfo,
		"Baz artist",
		"Bar artist",
		1,
	);
}

#[test_log::test]
fn remove_id3v2() {
	crate::util::remove_tag_test(
		"tests/files/assets/minimal/wav_format_pcm.wav",
		TagType::Id3v2,
	);
}

#[test_log::test]
fn remove_riff_info() {
	crate::util::remove_tag_test(
		"tests/files/assets/minimal/wav_format_pcm.wav",
		TagType::RiffInfo,
	);
}

#[test_log::test]
fn issue_174_divide_by_zero() {
	let file = Probe::open(
		"tests/files/assets/issue_174_waveformatextensible-ieeefloat-44100Hz-mono95060.wav",
	)
	.unwrap()
	.read()
	.unwrap();

	assert_eq!(file.file_type(), FileType::Wav);
}

#[test_log::test]
fn read_no_properties() {
	crate::util::no_properties_test("tests/files/assets/minimal/wav_format_pcm.wav");
}

#[test_log::test]
fn read_no_tags() {
	crate::util::no_tag_test("tests/files/assets/minimal/wav_format_pcm.wav", None);
}
