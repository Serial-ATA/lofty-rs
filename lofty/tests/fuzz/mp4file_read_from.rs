use crate::oom_test;
use lofty::config::ParseOptions;
use lofty::file::AudioFile;
use lofty::mp4::Mp4File;

#[test]
fn oom1() {
	oom_test::<Mp4File>("mp4file_read_from/oom-db2665d79ec9c045bdb9c1e9a3d0c93e7e59393e");
}

#[test]
fn panic1() {
	let mut reader = crate::get_reader(
		"mp4file_read_from/steam_at_mention_IDX_34_RAND_4491956654166691611931.m4a",
	);
	let _ = Mp4File::read_from(&mut reader, ParseOptions::new());
}

#[test]
fn panic2() {
	let mut reader = crate::get_reader(
		"mp4file_read_from/steam_at_mention_IDX_33_RAND_122808229373977607781108.m4a",
	);
	let _ = Mp4File::read_from(&mut reader, ParseOptions::new());
}

#[test]
fn panic3() {
	let mut reader = crate::get_reader(
		"mp4file_read_from/steam_at_mention_IDX_60_RAND_135276517902742448802109.m4a",
	);
	let _ = Mp4File::read_from(&mut reader, ParseOptions::new());
}
