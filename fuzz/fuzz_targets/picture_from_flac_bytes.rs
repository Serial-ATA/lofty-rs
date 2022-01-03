#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = lofty::Picture::from_flac_bytes(data, true);
    let _ = lofty::Picture::from_flac_bytes(data, false);
});