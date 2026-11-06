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
		if let Err(e) = common::init_corpus_dir("aifffile_read_from", &["aiff", "aifc"]) {
			eprintln!("{e}");
			return 1;
		}
	},
	|data: Vec<u8>| {
		let _ = lofty::iff::aiff::AiffFile::read_from(&mut Cursor::new(data), ParseOptions::new());
	}
);
