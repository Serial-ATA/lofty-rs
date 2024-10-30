use lofty::config::ParseOptions;
use lofty::file::AudioFile;
use lofty::musepack::MpcFile;

// Overflow when passing an AAC file to MpcFile::read_from
#[test_log::test]
fn panic1() {
	let mut reader = crate::get_reader("mpcfile_read_from/output.aac");
	let _ = MpcFile::read_from(&mut reader, ParseOptions::new());
}

// Overflow on badly sized ID3v2 tag
#[test_log::test]
fn panic2() {
	let mut reader =
		crate::get_reader("mpcfile_read_from/crash-c98d99ebce28b50b50eb2e96320f02e1e729e543");
	let _ = MpcFile::read_from(&mut reader, ParseOptions::new());
}
