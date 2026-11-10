use lofty::aac::AacFile;
use lofty::config::ParseOptions;
use lofty::file::AudioFile;

#[test_log::test]
fn panic1() {
	let mut reader = crate::get_reader(
		"aacfile_read_from/01 -  aalborg_IDX_9_RAND_168952727934877251846138.mp3",
	);
	let _ = AacFile::read_from(&mut reader, ParseOptions::new());
}

#[test_log::test]
fn timeout1() {
	crate::oom_test::<AacFile>(
		"aacfile_read_from/timeout-619f1587ebc7d02fd9dcfa1d1a3717b48a9f91fb",
	);
}
