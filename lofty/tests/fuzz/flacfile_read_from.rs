use crate::{get_reader, oom_test};
use lofty::config::ParseOptions;
use lofty::file::AudioFile;
use lofty::flac::FlacFile;

#[test_log::test]
fn oom1() {
	oom_test::<FlacFile>("flacfile_read_from/oom-9268264e9bc5e2124e4d63cbff8cff0b0dec6644");
}

#[test_log::test]
fn panic1() {
	let mut reader =
		get_reader("flacfile_read_from/flac_with_id3v2_IDX_39_RAND_108668567929800767822112.flac");
	let _ = FlacFile::read_from(&mut reader, ParseOptions::default());
}
