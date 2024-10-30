use crate::{get_reader, oom_test};
use lofty::config::ParseOptions;
use lofty::mpeg::MpegFile;
use lofty::prelude::*;

#[test_log::test]
fn crash1() {
	let mut reader =
		get_reader("mpegfile_read_from/crash-9b17818b6404b1c4b9f89c09dc11e915b96cafc6");

	let _ = MpegFile::read_from(&mut reader, ParseOptions::new());
}

#[test_log::test]
fn crash2() {
	let mut reader =
		get_reader("mpegfile_read_from/crash-718f75611e77caac968c7f68cdefa1472172f64b");

	let _ = MpegFile::read_from(&mut reader, ParseOptions::new());
}

#[test_log::test]
fn crash3() {
	let mut reader =
		get_reader("mpegfile_read_from/crash-7b5eb183cc14faf3ecc93bdf4a5e30b05f7a537d_minimized");
	let _ = MpegFile::read_from(&mut reader, ParseOptions::new());
}

#[test_log::test]
fn oom1() {
	oom_test::<MpegFile>("mpegfile_read_from/oom-f8730cbfa5682ab12343ccb70de9b71a061ef4d0");
}
