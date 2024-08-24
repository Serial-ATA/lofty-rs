use lofty::config::ParseOptions;
use lofty::file::AudioFile;
use lofty::musepack::MpcFile;

// Overflow when passing an AAC file to MpcFile::read_from
#[test]
fn panic1() {
	let mut reader = crate::get_reader("mpcfile_read_from/output.aac");
	let _ = MpcFile::read_from(&mut reader, ParseOptions::new());
}
