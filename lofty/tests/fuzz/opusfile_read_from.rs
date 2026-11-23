use crate::oom_test;
use lofty::config::ParseOptions;
use lofty::file::AudioFile;
use lofty::ogg::OpusFile;

#[test_log::test]
fn oom1() {
	oom_test::<OpusFile>("opusfile_read_from/oom-7126e68a5a9ef53351c46f3c55b7e1a582705fcc");
}

// Overflow when converting a huge granule position to a duration
#[test_log::test]
fn panic1() {
	let mut reader =
		crate::get_reader("opusfile_read_from/crash-aa32aebbb05c6e37d1e690bddbc5bdc77ee99a06");
	let _ = OpusFile::read_from(&mut reader, ParseOptions::new().read_tags(false));
}

// Division by zero when the duration rounds down to 0ms
#[test_log::test]
fn panic2() {
	let mut reader =
		crate::get_reader("opusfile_read_from/crash-4b996e995a9692795e28fbae6df0050b9fc3591b");
	let _ = OpusFile::read_from(&mut reader, ParseOptions::new());
}

// Zero channels with mapping family 0
#[test_log::test]
fn panic3() {
	let mut reader =
		crate::get_reader("opusfile_read_from/crash-54a4e05eb26ff141da8dc9a67b170857ae8aa2a6");
	let _ = OpusFile::read_from(&mut reader, ParseOptions::new());
}
