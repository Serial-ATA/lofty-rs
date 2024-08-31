#![no_main]

use libfuzzer_sys::fuzz_target;
use lofty::config::ParsingMode;

fuzz_target!(|data: &[u8]| {
	let _ = lofty::picture::Picture::from_flac_bytes(data, true, ParsingMode::Relaxed);
	let _ = lofty::picture::Picture::from_flac_bytes(data, false, ParsingMode::Relaxed);
});
