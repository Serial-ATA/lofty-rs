#![no_main]
use libfuzzer_sys::fuzz_target;
use lofty::id3::v2::ID3v2Version;

fuzz_target!(|data: &[u8]| {
	let _ = lofty::Picture::from_apic_bytes(data, ID3v2Version::V4);
});
