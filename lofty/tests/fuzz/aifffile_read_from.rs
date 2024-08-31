use crate::{get_reader, oom_test};
use lofty::config::ParseOptions;
use lofty::file::AudioFile;
use lofty::iff::aiff::AiffFile;

#[test_log::test]
fn oom1() {
	oom_test::<AiffFile>("aifffile_read_from/oom-88065007d35ee271d5812fd723a3b458488813ea");
}

#[test_log::test]
fn panic1() {
	let mut reader =
		get_reader("aifffile_read_from/full_test_IDX_5_RAND_89430166450532348786207.aiff");
	let _ = AiffFile::read_from(&mut reader, ParseOptions::default());
}
