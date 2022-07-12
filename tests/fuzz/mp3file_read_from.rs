use crate::oom_test;
use lofty::mp3::Mp3File;

#[test]
fn oom1() {
	oom_test::<Mp3File>("mp3file_read_from/oom-f8730cbfa5682ab12343ccb70de9b71a061ef4d0");
}
