use crate::oom_test;
use lofty::ogg::OpusFile;

#[test_log::test]
fn oom1() {
	oom_test::<OpusFile>("opusfile_read_from/oom-7126e68a5a9ef53351c46f3c55b7e1a582705fcc");
}
