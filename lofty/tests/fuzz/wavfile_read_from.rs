use crate::oom_test;
use lofty::config::ParseOptions;
use lofty::file::AudioFile;
use lofty::iff::wav::WavFile;

#[test]
fn oom1() {
	oom_test::<WavFile>("wavfile_read_from/oom-007573d233b412ea1b8038137db28e70d5678291");
}

#[test]
fn panic1() {
	let mut reader =
		crate::get_reader("wavfile_read_from/2_IDX_0_RAND_85629492689553753214598.wav");
	let _ = WavFile::read_from(&mut reader, ParseOptions::new());
}
