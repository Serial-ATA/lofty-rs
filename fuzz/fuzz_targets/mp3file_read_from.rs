#![no_main]

use std::io::Cursor;

use lofty::AudioFile;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: Vec<u8>| {
    let _ = lofty::mp3::Mp3File::read_from(&mut Cursor::new(data), false);
});
