use std::{
	fs::{self, File},
	path::Path,
};

use hound::WavReader;
use lofty::{iff::WavFile, AudioFile};

fn get_properties(path: &Path) -> <lofty::iff::WavFile as AudioFile>::Properties {
	let mut f = File::open(path).unwrap();
	let wav_file = WavFile::read_from(&mut f, true).unwrap();
	*wav_file.properties()
}

#[test]
fn hound() {
	let paths = fs::read_dir("tests/files/assets/hound").unwrap();

	for path in paths {
		let path = path.unwrap().path();
		if path.is_file() && path.extension().unwrap() == "wav" {
			println!("Name: {}", path.display());
			let wav_reader = WavReader::open(&path).unwrap();
			let lofty = get_properties(&path);
			assert_eq!(wav_reader.spec().channels, lofty.channels() as u16);
			assert_eq!(wav_reader.spec().sample_rate, lofty.sample_rate());
			assert_eq!(wav_reader.spec().bits_per_sample, lofty.bit_depth() as u16);
		}
	}
}
