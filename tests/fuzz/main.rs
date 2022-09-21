use lofty::{AudioFile, ParseOptions};
use std::io::Cursor;
use std::path::Path;
use std::thread;
use std::time::Instant;

mod aifffile_read_from;
mod flacfile_read_from;
mod mp4file_read_from;
mod mpegfile_read_from;
mod opusfile_read_from;
mod pictureinformation_from_jpeg;
mod pictureinformation_from_png;
mod speexfile_read_from;
mod vorbisfile_read_from;
mod wavfile_read_from;
mod wavpackfile_read_from;

pub fn get_reader(path: &str) -> Cursor<Vec<u8>> {
	let path = Path::new("tests/fuzz/assets").join(path);

	let b = std::fs::read(path).unwrap();
	Cursor::new(b)
}

pub fn oom_test<A: AudioFile>(path: &'static str) {
	let instant = Instant::now();
	let thread = thread::spawn(|| {
		let _ = <A as AudioFile>::read_from(&mut get_reader(path), ParseOptions::new());
	});

	while instant.elapsed().as_secs() < 3 {
		if thread.is_finished() {
			return;
		}
	}

	panic!("Failed to run test");
}
