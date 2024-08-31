use lofty::ape::ApeFile;
use lofty::config::{ParseOptions, ParsingMode};
use lofty::flac::FlacFile;
use lofty::iff::aiff::AiffFile;
use lofty::iff::wav::WavFile;
use lofty::mp4::Mp4File;
use lofty::mpeg::MpegFile;
use lofty::prelude::*;

fn read_file_with_properties<A: AudioFile>(path: &str) -> bool {
	let res = <A as AudioFile>::read_from(
		&mut std::fs::File::open(path).unwrap(),
		ParseOptions::new().parsing_mode(ParsingMode::Strict),
	);
	res.is_ok()
}

fn read_file_no_properties<A: AudioFile>(path: &str) -> bool {
	let res = <A as AudioFile>::read_from(
		&mut std::fs::File::open(path).unwrap(),
		ParseOptions::new().read_properties(false),
	);
	res.is_ok()
}

#[test_log::test]
fn zero_audio_aiff() {
	let path = "tests/files/assets/zero/zero.aiff";

	// An AIFF files with a zero-size SSND chunk will error when attempting to read properties
	assert!(!read_file_with_properties::<AiffFile>(path));

	assert!(read_file_no_properties::<AiffFile>(path));
}

#[test_log::test]
fn zero_audio_ape() {
	let path = "tests/files/assets/zero/zero.ape";

	// An APE file with total_frames = 0 will error when attempting to read properties
	assert!(!read_file_with_properties::<ApeFile>(path));

	assert!(read_file_no_properties::<ApeFile>(path))
}

#[test_log::test]
fn zero_audio_flac() {
	let path = "tests/files/assets/zero/zero.flac";
	assert!(read_file_with_properties::<FlacFile>(path));
	assert!(read_file_no_properties::<FlacFile>(path));
}

#[test_log::test]
fn zero_audio_mp3() {
	let path = "tests/files/assets/zero/zero.mp3";
	// A zero-size MP3 will error, since we need MPEG frames to extract audio properties
	assert!(!read_file_with_properties::<MpegFile>(path));

	assert!(read_file_no_properties::<MpegFile>(path))
}

#[test_log::test]
fn zero_audio_mp4() {
	let path = "tests/files/assets/zero/zero.mp4";

	// A zero-size MP4 will error, since we need an audio track to extract audio properties
	assert!(!read_file_with_properties::<Mp4File>(path));

	assert!(read_file_no_properties::<Mp4File>(path))
}

// zero-size Vorbis, Opus, and Speex files are invalid

#[test_log::test]
fn zero_audio_wav() {
	let path = "tests/files/assets/zero/zero.wav";
	// An empty "data" chunk is an error
	assert!(!read_file_with_properties::<WavFile>(path));

	assert!(read_file_no_properties::<WavFile>(path));
}
