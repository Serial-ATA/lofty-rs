use crate::oom_test;
use lofty::flac::FlacFile;

#[test]
fn oom1() {
	oom_test::<FlacFile>("flacfile_read_from/oom-9268264e9bc5e2124e4d63cbff8cff0b0dec6644");
}
