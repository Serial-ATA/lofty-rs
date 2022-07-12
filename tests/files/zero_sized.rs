use lofty::ape::ApeFile;
use lofty::flac::FlacFile;
use lofty::iff::{AiffFile, WavFile};
use lofty::mp3::Mp3File;
use lofty::mp4::Mp4File;
use lofty::AudioFile;

fn read_file_with_properties<A: AudioFile>(path: &str) -> bool {
	let res = <A as AudioFile>::read_from(&mut std::fs::File::open(path).unwrap(), true);
	res.is_ok()
}

fn read_file_no_properties<A: AudioFile>(path: &str) -> bool {
	let res = <A as AudioFile>::read_from(&mut std::fs::File::open(path).unwrap(), false);
	res.is_ok()
}

#[test]
fn zero_audio_aiff() {
	let path = "tests/files/assets/zero/zero.aiff";

	// An AIFF files with a zero-size SSND chunk will error when attempting to read properties
	assert!(!read_file_with_properties::<AiffFile>(path));

	assert!(read_file_no_properties::<AiffFile>(path));
}

#[test]
fn zero_audio_ape() {
	let path = "tests/files/assets/zero/zero.ape";

	// An APE file with total_frames = 0 will error when attempting to read properties
	assert!(!read_file_with_properties::<ApeFile>(path));

	assert!(read_file_no_properties::<ApeFile>(path))
}

#[test]
fn zero_audio_flac() {
	let path = "tests/files/assets/zero/zero.flac";
	assert!(read_file_with_properties::<FlacFile>(path));
	assert!(read_file_no_properties::<FlacFile>(path));
}

#[test]
fn zero_audio_mp3() {
	let path = "tests/files/assets/zero/zero.mp3";
	// A zero-size MP3 will error, since we need MPEG frames to extract audio properties
	assert!(!read_file_with_properties::<Mp3File>(path));

	assert!(read_file_no_properties::<Mp3File>(path))
}

#[test]
fn zero_audio_mp4() {
	let path = "tests/files/assets/zero/zero.mp4";

	// A zero-size MP4 will error, since we need an audio track to extract audio properties
	assert!(!read_file_with_properties::<Mp4File>(path));

	assert!(read_file_no_properties::<Mp4File>(path))
}

// zero-size Vorbis, Opus, and Speex files are invalid

#[test]
fn zero_audio_wav() {
	let path = "tests/files/assets/zero/zero.wav";
	// An empty "data" chunk is an error
	assert!(!read_file_with_properties::<WavFile>(path));

	assert!(read_file_no_properties::<WavFile>(path));
}
