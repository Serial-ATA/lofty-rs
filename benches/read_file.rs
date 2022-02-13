use lofty::Probe;

use criterion::{criterion_group, criterion_main, Criterion};

use std::io::Cursor;

macro_rules! test_read_path {
	($c:ident, [$(($NAME:literal, $path:expr)),+]) => {
		let mut g = $c.benchmark_group("File reading (Inferred from Path)");

		$(
			g.bench_function($NAME, |b| b.iter(|| Probe::open($path).unwrap().read(true).unwrap()));
		)+
	};
}

fn path_infer_read(c: &mut Criterion) {
	test_read_path!(
		c,
		[
			("AIFF", "tests/files/assets/minimal/full_test.aiff"),
			("APE", "tests/files/assets/minimal/full_test.ape"),
			("FLAC", "tests/files/assets/minimal/full_test.flac"),
			("MP4", "tests/files/assets/minimal/m4a_codec_aac.m4a"),
			("MP3", "tests/files/assets/minimal/full_test.mp3"),
			("OPUS", "tests/files/assets/minimal/full_test.opus"),
			("RIFF", "tests/files/assets/minimal/wav_format_pcm.wav"),
			("SPEEX", "tests/files/assets/minimal/full_test.spx"),
			("VORBIS", "tests/files/assets/minimal/full_test.ogg")
		]
	);
}

macro_rules! test_read_file {
	($c:ident, [$(($NAME:ident, $path:expr)),+]) => {
		let mut g = $c.benchmark_group("File reading (Inferred from Content)");

		$(
			const $NAME: &[u8] = include_bytes!($path);

			g.bench_function(
				stringify!($NAME),
				|b| b.iter(|| {
					Probe::new(Cursor::new($NAME))
					.guess_file_type()
					.unwrap()
					.read(true)
					.unwrap()
				})
			);
		)+
	}
}

fn content_infer_read(c: &mut Criterion) {
	test_read_file!(
		c,
		[
			(AIFF, "../tests/files/assets/minimal/full_test.aiff"),
			(APE, "../tests/files/assets/minimal/full_test.ape"),
			(FLAC, "../tests/files/assets/minimal/full_test.flac"),
			(MP4, "../tests/files/assets/minimal/m4a_codec_aac.m4a"),
			(MP3, "../tests/files/assets/minimal/full_test.mp3"),
			(OPUS, "../tests/files/assets/minimal/full_test.opus"),
			(RIFF, "../tests/files/assets/minimal/wav_format_pcm.wav"),
			(SPEEX, "../tests/files/assets/minimal/full_test.spx"),
			(VORBIS, "../tests/files/assets/minimal/full_test.ogg")
		]
	);
}

criterion_group!(benches, path_infer_read, content_infer_read);
criterion_main!(benches);
