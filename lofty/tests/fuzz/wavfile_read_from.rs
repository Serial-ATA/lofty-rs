use crate::oom_test;
use lofty::config::ParseOptions;
use lofty::file::AudioFile;
use lofty::iff::wav::WavFile;

#[test_log::test]
fn oom1() {
	oom_test::<WavFile>("wavfile_read_from/oom-007573d233b412ea1b8038137db28e70d5678291");
}

#[test_log::test]
fn panic1() {
	let mut reader =
		crate::get_reader("wavfile_read_from/2_IDX_0_RAND_85629492689553753214598.wav");
	let _ = WavFile::read_from(&mut reader, ParseOptions::new());
}

#[test_log::test]
fn panic2() {
	let mut reader =
		crate::get_reader("wavfile_read_from/2_IDX_63_RAND_104275228651573584855676.wav");
	let _ = WavFile::read_from(&mut reader, ParseOptions::new());
}

#[test_log::test]
fn panic3() {
	let mut reader =
		crate::get_reader("wavfile_read_from/2_IDX_34_RAND_128635499166458268533001.wav");
	let _ = WavFile::read_from(&mut reader, ParseOptions::new());
}

#[test_log::test]
fn panic4() {
	let mut reader = crate::get_reader("wavfile_read_from/aa");
	let _ = WavFile::read_from(&mut reader, ParseOptions::new());
}
