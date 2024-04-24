use crate::oom_test;
use lofty::wavpack::WavPackFile;

#[test]
fn oom1() {
	oom_test::<WavPackFile>(
		"wavpackfile_read_from/minimized-from-1e67c08d7f69bc3ac39aeeede515b96fffcb31b4",
	);
}

#[test]
fn oom2() {
	oom_test::<WavPackFile>(
		"wavpackfile_read_from/minimized-from-3f74da5ead463d922c1de1f57ad7ac9697e3f79d",
	);
}

#[test]
fn oom3() {
	oom_test::<WavPackFile>(
		"wavpackfile_read_from/minimized-from-7eae56cca38a302e693fcbc3853798f6298c5e90",
	);
}

#[test]
fn oom4() {
	oom_test::<WavPackFile>(
		"wavpackfile_read_from/minimized-from-7f1d89b3c498ff9a180cfdb85ab3b51f25756991",
	);
}

#[test]
fn oom5() {
	oom_test::<WavPackFile>(
		"wavpackfile_read_from/minimized-from-56e6e9ffb1642607fb9aba7f7613667882a4fd0c",
	);
}

#[test]
fn oom6() {
	oom_test::<WavPackFile>(
		"wavpackfile_read_from/minimized-from-68b3837442b17e87863f02299e0cce1c4145c76b",
	);
}

#[test]
fn oom7() {
	oom_test::<WavPackFile>(
		"wavpackfile_read_from/minimized-from-94867b6fefcd32cd5bc3bc298468cd3d65d93ff1",
	);
}

#[test]
fn oom8() {
	oom_test::<WavPackFile>(
		"wavpackfile_read_from/minimized-from-625824728fdaa4cbc0acb6e58a2737f60c7446f8",
	);
}

#[test]
fn oom9() {
	oom_test::<WavPackFile>(
		"wavpackfile_read_from/minimized-from-aa6f3592d16b7845dea49b6f261e4c6fbd9a2143",
	);
}

#[test]
fn oom10() {
	oom_test::<WavPackFile>(
		"wavpackfile_read_from/minimized-from-cdb6b62e519b2f42c6c376ad125679c83a11f6cf",
	);
}

#[test]
fn oom11() {
	oom_test::<WavPackFile>(
		"wavpackfile_read_from/minimized-from-e08dd883f7816664aa627662e0674706b47e76db",
	);
}

#[test]
fn oom12() {
	oom_test::<WavPackFile>("wavpackfile_read_from/oom-94867b6fefcd32cd5bc3bc298468cd3d65d93ff1");
}
