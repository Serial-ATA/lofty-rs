use crate::oom_test;
use lofty::iff::WavFile;

#[test]
fn oom1() {
	oom_test::<WavFile>("wavfile_read_from/oom-007573d233b412ea1b8038137db28e70d5678291");
}
