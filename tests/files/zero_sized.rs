use lofty::ape::ApeFile;
use lofty::flac::FlacFile;
use lofty::iff::{AiffFile, WavFile};
use lofty::mp3::Mp3File;
use lofty::mp4::Mp4File;
use lofty::AudioFile;

// TODO: zero-size mdat mp4
// TODO: zero-size vorbis comments
// TODO: zero-size APE tag
// TODO: zero-size ilst
// TODO: zero-size AIFF text chunks

fn read_file<A: AudioFile + std::fmt::Debug>(path: &str) -> bool {
	let res = <A as AudioFile>::read_from(&mut std::fs::File::open(path).unwrap(), true);
	res.is_ok()
}

#[test]
fn zero_audio_aiff() {
	// An AIFF files with a zero-size SSND chunk will error when attempting to read properties
	assert!(!read_file::<AiffFile>("tests/files/assets/zero/zero.aiff"));
}

#[test]
fn zero_audio_ape() {
	// An APE file with total_frames = 0 will error when attempting to read properties
	assert!(!read_file::<ApeFile>("tests/files/assets/zero/zero.ape"));
}

#[test]
fn zero_audio_flac() {
	assert!(read_file::<FlacFile>("tests/files/assets/zero/zero.flac"));
}

#[test]
fn zero_audio_mp3() {
	// A zero-size MP3 will error, since we need MPEG frames to extract audio properties
	assert!(!read_file::<Mp3File>("tests/files/assets/zero/zero.mp3"));
}

#[test]
fn zero_audio_mp4() {
	// A zero-size MP4 will error, since we need an audio track to extract audio properties
	assert!(!read_file::<Mp4File>("tests/files/assets/zero/zero.mp4"));
}

// zero-size Vorbis, Opus, and Speex files are invalid

#[test]
fn zero_audio_wav() {
	// An empty "data" chunk is an error
	assert!(!read_file::<WavFile>("tests/files/assets/zero/zero.wav"));
}
