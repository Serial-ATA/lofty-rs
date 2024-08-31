#![no_main]

use std::io::Cursor;

use libfuzzer_sys::fuzz_target;
use lofty::probe::Probe;

fuzz_target!(|data: Vec<u8>| {
	if let Ok(probe) = Probe::new(&mut Cursor::new(data)).guess_file_type() {
		let _ = probe.read();
	}
});
