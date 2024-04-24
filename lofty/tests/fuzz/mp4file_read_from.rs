use crate::oom_test;
use lofty::mp4::Mp4File;

#[test]
fn oom1() {
	oom_test::<Mp4File>("mp4file_read_from/oom-db2665d79ec9c045bdb9c1e9a3d0c93e7e59393e");
}
