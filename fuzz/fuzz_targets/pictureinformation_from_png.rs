#![allow(missing_docs)]
#![no_main]

#[path = "../common.rs"]
mod common;

use libfuzzer_sys::fuzz_target;

fuzz_target!(
	init: {
		if let Err(e) = common::init_corpus_dir("pictureinformation_from_png", &["png"]) {
			eprintln!("{e}");
			return 1;
		}
	},
	|data: &[u8]| {
		let _ = lofty::picture::PictureInformation::from_png(data);
	}
);
