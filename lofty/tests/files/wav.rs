use lofty::config::{ParseOptions, WriteOptions};
use lofty::file::FileType;
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::TagType;

use std::io::{Read, Seek};

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
fn growing_trailing_id3v2_updates_stream_size() {
	let mut file = crate::util::temp_file("tests/files/assets/minimal/wav_format_pcm.wav");
	TagType::RiffInfo
		.remove_from(&mut file, WriteOptions::default())
		.unwrap();
	file.rewind().unwrap();

	let mut tagged_file = Probe::new(file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read_bound()
		.unwrap();
	tagged_file
		.tag_mut(TagType::Id3v2)
		.unwrap()
		.set_artist("A much longer artist name".repeat(100));
	tagged_file.save(WriteOptions::default()).unwrap();

	let mut file = tagged_file.into_inner();
	let file_len = file.metadata().unwrap().len();
	file.rewind().unwrap();
	let mut header = [0; 8];
	file.read_exact(&mut header).unwrap();
	assert_eq!(
		u64::from(u32::from_le_bytes(header[4..8].try_into().unwrap())) + 8,
		file_len
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

#[test_log::test]
fn empty_id3_chunk_skipped_644() {
	let file = Probe::open("tests/files/assets/644_empty_id3v2.wav")
		.unwrap()
		.read()
		.unwrap();

	assert_eq!(file.file_type(), FileType::Wav);
	assert!(file.tag(TagType::Id3v2).is_none());
}
