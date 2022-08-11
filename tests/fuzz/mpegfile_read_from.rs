use crate::oom_test;
use lofty::mpeg::MPEGFile;

#[test]
fn oom1() {
	oom_test::<MPEGFile>("mpegfile_read_from/oom-f8730cbfa5682ab12343ccb70de9b71a061ef4d0");
}
