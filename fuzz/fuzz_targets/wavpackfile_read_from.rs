#![allow(missing_docs)]
#![no_main]

#[path = "../common.rs"]
mod common;

use std::io::Cursor;

use libfuzzer_sys::fuzz_target;
use lofty::config::ParseOptions;
use lofty::file::AudioFile;

fuzz_target!(
	init: {
		if let Err(e) = common::init_corpus_dir("wavpackfile_read_from", &["wv"]) {
			eprintln!("{e}");
			return 1;
		}
	},
	|data: Vec<u8>| {
		let _ = lofty::wavpack::WavPackFile::read_from(&mut Cursor::new(data), ParseOptions::new());
	}
);
