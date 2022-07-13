use crate::oom_test;
use lofty::ogg::VorbisFile;

#[test]
fn oom1() {
	oom_test::<VorbisFile>("vorbisfile_read_from/oom-436193bc2d1664b74c19720bef08697d03284f06");
}
