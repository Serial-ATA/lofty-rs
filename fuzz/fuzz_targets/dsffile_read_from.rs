#![no_main]

use std::io::Cursor;

use libfuzzer_sys::fuzz_target;
use lofty::config::ParseOptions;
use lofty::file::AudioFile;

fuzz_target!(|data: Vec<u8>| {
	let _ = lofty::dsd::dsf::DsfFile::read_from(
		&mut Cursor::new(data),
		ParseOptions::new().read_properties(false),
	);
});
