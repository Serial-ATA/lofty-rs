use crate::oom_test;
use lofty::config::ParseOptions;
use lofty::file::AudioFile;
use lofty::wavpack::WavPackFile;

#[test_log::test]
fn oom1() {
	oom_test::<WavPackFile>(
		"wavpackfile_read_from/minimized-from-1e67c08d7f69bc3ac39aeeede515b96fffcb31b4",
	);
}

#[test_log::test]
fn oom2() {
	oom_test::<WavPackFile>(
		"wavpackfile_read_from/minimized-from-3f74da5ead463d922c1de1f57ad7ac9697e3f79d",
	);
}

#[test_log::test]
fn oom3() {
	oom_test::<WavPackFile>(
		"wavpackfile_read_from/minimized-from-7eae56cca38a302e693fcbc3853798f6298c5e90",
	);
}

#[test_log::test]
fn oom4() {
	oom_test::<WavPackFile>(
		"wavpackfile_read_from/minimized-from-7f1d89b3c498ff9a180cfdb85ab3b51f25756991",
	);
}

#[test_log::test]
fn oom5() {
	oom_test::<WavPackFile>(
		"wavpackfile_read_from/minimized-from-56e6e9ffb1642607fb9aba7f7613667882a4fd0c",
	);
}

#[test_log::test]
fn oom6() {
	oom_test::<WavPackFile>(
		"wavpackfile_read_from/minimized-from-68b3837442b17e87863f02299e0cce1c4145c76b",
	);
}

#[test_log::test]
fn oom7() {
	oom_test::<WavPackFile>(
		"wavpackfile_read_from/minimized-from-94867b6fefcd32cd5bc3bc298468cd3d65d93ff1",
	);
}

#[test_log::test]
fn oom8() {
	oom_test::<WavPackFile>(
		"wavpackfile_read_from/minimized-from-625824728fdaa4cbc0acb6e58a2737f60c7446f8",
	);
}

#[test_log::test]
fn oom9() {
	oom_test::<WavPackFile>(
		"wavpackfile_read_from/minimized-from-aa6f3592d16b7845dea49b6f261e4c6fbd9a2143",
	);
}

#[test_log::test]
fn oom10() {
	oom_test::<WavPackFile>(
		"wavpackfile_read_from/minimized-from-cdb6b62e519b2f42c6c376ad125679c83a11f6cf",
	);
}

#[test_log::test]
fn oom11() {
	oom_test::<WavPackFile>(
		"wavpackfile_read_from/minimized-from-e08dd883f7816664aa627662e0674706b47e76db",
	);
}

#[test_log::test]
fn oom12() {
	oom_test::<WavPackFile>("wavpackfile_read_from/oom-94867b6fefcd32cd5bc3bc298468cd3d65d93ff1");
}

#[test_log::test]
fn panic1() {
	let mut reader = crate::get_reader("wavpackfile_read_from/output");
	let _ = WavPackFile::read_from(&mut reader, ParseOptions::default());
}

#[test_log::test]
fn panic2() {
	let mut reader = crate::get_reader("wavpackfile_read_from/bb");
	let _ = WavPackFile::read_from(&mut reader, ParseOptions::default());
}

#[test_log::test]
fn panic3() {
	let mut reader = crate::get_reader(
		"wavpackfile_read_from/crash-c6f0765886234e3a25b182f01bc3f92880188f5b_minimized",
	);
	let _ = WavPackFile::read_from(&mut reader, ParseOptions::default());
}

#[test_log::test]
fn panic4() {
	let mut reader = crate::get_reader(
		"wavpackfile_read_from/crash-96407368cf46fbf0ef1285c4d84fbd39a919ef2b_minimized",
	);
	let _ = WavPackFile::read_from(&mut reader, ParseOptions::default());
}

#[test_log::test]
fn panic5() {
	let mut reader = crate::get_reader(
		"wavpackfile_read_from/crash-5f9ecf40152ed0dcb39eb66003ecca7d42d56bf3_minimized",
	);
	let _ = WavPackFile::read_from(&mut reader, ParseOptions::default());
}

#[test_log::test]
fn panic6() {
	let mut reader = crate::get_reader(
		"wavpackfile_read_from/crash-68a2215c732ecb202998d3bd8b0de932e5e0301d_minimized",
	);
	let _ = WavPackFile::read_from(&mut reader, ParseOptions::default());
}

#[test_log::test]
fn panic7() {
	let mut reader = crate::get_reader(
		"wavpackfile_read_from/crash-b583ce7029fc17100e2aabfa4679865a2a5fd9a4_minimized",
	);
	let _ = WavPackFile::read_from(&mut reader, ParseOptions::default());
}
