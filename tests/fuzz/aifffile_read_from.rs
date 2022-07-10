use crate::oom_test;
use lofty::iff::AiffFile;

#[test]
fn oom1() {
	oom_test::<AiffFile>("aifffile_read_from/oom-88065007d35ee271d5812fd723a3b458488813ea");
}
