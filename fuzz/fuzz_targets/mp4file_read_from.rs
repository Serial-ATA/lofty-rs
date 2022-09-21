#![no_main]

use std::io::Cursor;

use libfuzzer_sys::fuzz_target;
use lofty::{AudioFile, ParseOptions};

fuzz_target!(|data: Vec<u8>| {
	let _ = lofty::mp4::Mp4File::read_from(
		&mut Cursor::new(data),
		ParseOptions::new().read_properties(false),
	);
});
