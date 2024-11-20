#![allow(missing_docs)]

use lofty::config::ParseOptions;
use lofty::error::Result;
use lofty::iff::wav::WavFile;
use lofty::prelude::*;

use hound::WavReader;

use std::fs;
use std::fs::File;
use std::path::Path;

fn get_properties(path: &Path) -> Result<<WavFile as AudioFile>::Properties> {
	let mut f = File::open(path).unwrap();
	let wav_file = WavFile::read_from(&mut f, ParseOptions::new())?;
	Ok(*wav_file.properties())
}

#[test_log::test]
fn hound() {
	let paths = fs::read_dir("tests/files/assets/hound").unwrap();

	for path in paths {
		let path = path.unwrap().path();
		if path.is_file() && path.extension().unwrap() == "wav" {
			println!("Name: {}", path.display());
			let wav_reader = WavReader::open(&path).unwrap();
			let lofty = get_properties(&path).unwrap();
			assert_eq!(u16::from(lofty.channels()), wav_reader.spec().channels);
			assert_eq!(lofty.sample_rate(), wav_reader.spec().sample_rate);
			assert_eq!(
				u16::from(lofty.bit_depth()),
				wav_reader.spec().bits_per_sample
			);
		}
	}
}

#[test_log::test]
fn hound_fuzz() {
	let paths = fs::read_dir("tests/files/assets/hound/fuzz").unwrap();

	for path in paths {
		let path = path.unwrap().path();
		if path.is_file() && path.extension().unwrap() == "wav" {
			println!("Name: {}", path.display());
			if let Ok(wav_reader) = WavReader::open(&path) {
				let lofty = get_properties(&path).unwrap();
				println!("{lofty:#?}");
				assert_eq!(u16::from(lofty.channels()), wav_reader.spec().channels);
				assert_eq!(lofty.sample_rate(), wav_reader.spec().sample_rate);
				assert_eq!(
					u16::from(lofty.bit_depth()),
					wav_reader.spec().bits_per_sample
				);
			} else if get_properties(&path).is_ok() {
				println!("We are even better for this file!");
			}
		}
	}
}
