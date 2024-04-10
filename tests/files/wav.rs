use crate::{set_artist, temp_file, verify_artist};
use lofty::prelude::*;
use lofty::{FileType, ItemKey, ItemValue, ParseOptions, Probe, TagItem, TagType, TaggedFileExt};

use std::io::{Seek, Write};

#[test]
fn read() {
	// Here we have a WAV file with both an ID3v2 chunk and a RIFF INFO chunk
	let file = Probe::open("tests/files/assets/minimal/wav_format_pcm.wav")
		.unwrap()
		.options(ParseOptions::new().read_properties(false))
		.read()
		.unwrap();

	assert_eq!(file.file_type(), FileType::Wav);

	// Verify the ID3v2 tag first
	crate::verify_artist!(file, primary_tag, "Foo artist", 1);

	// Now verify the RIFF INFO chunk
	crate::verify_artist!(file, tag, TagType::RiffInfo, "Bar artist", 1);
}

#[test]
fn write() {
	let mut file = temp_file!("tests/files/assets/minimal/wav_format_pcm.wav");

	let mut tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	assert_eq!(tagged_file.file_type(), FileType::Wav);

	// ID3v2
	crate::set_artist!(tagged_file, primary_tag_mut, "Foo artist", 1 => file, "Bar artist");

	// RIFF INFO
	crate::set_artist!(tagged_file, tag_mut, TagType::RiffInfo, "Bar artist", 1 => file, "Baz artist");

	// Now reread the file
	file.rewind().unwrap();
	let mut tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	crate::set_artist!(tagged_file, primary_tag_mut, "Bar artist", 1 => file, "Foo artist");

	crate::set_artist!(tagged_file, tag_mut, TagType::RiffInfo, "Baz artist", 1 => file, "Bar artist");
}

#[test]
fn remove_id3v2() {
	crate::remove_tag!(
		"tests/files/assets/minimal/wav_format_pcm.wav",
		TagType::Id3v2
	);
}

#[test]
fn remove_riff_info() {
	crate::remove_tag!(
		"tests/files/assets/minimal/wav_format_pcm.wav",
		TagType::RiffInfo
	);
}

#[test]
fn issue_174_divide_by_zero() {
	let file = Probe::open(
		"tests/files/assets/issue_174_waveformatextensible-ieeefloat-44100Hz-mono95060.wav",
	)
	.unwrap()
	.read()
	.unwrap();

	assert_eq!(file.file_type(), FileType::Wav);
}
