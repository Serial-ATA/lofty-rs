use lofty::ape::ApeFile;
use lofty::config::ParseOptions;
use lofty::file::AudioFile;

#[test_log::test]
fn panic1() {
	let mut reader = crate::get_reader(
		"apefile_read_from/crash-6373119c37ca5982277fc75787a0a3c34aadbca7_minimized",
	);
	let _ = ApeFile::read_from(&mut reader, ParseOptions::default());
}
