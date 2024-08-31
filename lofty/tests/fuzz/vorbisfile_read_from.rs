use crate::oom_test;
use lofty::config::ParseOptions;
use lofty::file::AudioFile;
use lofty::ogg::VorbisFile;

#[test_log::test]
fn oom1() {
	oom_test::<VorbisFile>("vorbisfile_read_from/oom-436193bc2d1664b74c19720bef08697d03284f06");
}

#[test_log::test]
fn large_allocation() {
	let mut reader =
		crate::get_reader("vorbisfile_read_from/move01d_IDX_13_RAND_35154275996070165946691.ogg");
	let _ = VorbisFile::read_from(&mut reader, ParseOptions::new());
}

#[test_log::test]
fn panic1() {
	let mut reader =
		crate::get_reader("vorbisfile_read_from/order01d_IDX_32_RAND_22064097693866277502540.ogg");
	let _ = VorbisFile::read_from(&mut reader, ParseOptions::new());
}
