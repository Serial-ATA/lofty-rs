use crate::oom_test;
use lofty::ogg::SpeexFile;

#[test_log::test]
fn oom1() {
	oom_test::<SpeexFile>("speexfile_read_from/oom-7976a4c57e7f8b4ac428f9e7f846b59d2dce714f");
}
