use lofty::{ParseOptions, Probe};

use criterion::{criterion_group, criterion_main, Criterion};

use std::io::Cursor;

macro_rules! test_read_file {
	($c:ident, [$(($NAME:ident, $path:expr)),+ $(,)?]) => {
		let mut g = $c.benchmark_group("File reading (Inferred from Content)");

		$(
			const $NAME: &[u8] = include_bytes!($path);

			g.bench_function(
				stringify!($NAME),
				|b| b.iter(|| {
					Probe::new(Cursor::new($NAME))
					.options(ParseOptions::new())
					.guess_file_type()
					.unwrap()
					.read()
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
			(AAC, "../tests/files/assets/minimal/full_test.aac"),
			(AIFF, "../tests/files/assets/minimal/full_test.aiff"),
			(APE, "../tests/files/assets/minimal/full_test.ape"),
			(FLAC, "../tests/files/assets/minimal/full_test.flac"),
			(MP4, "../tests/files/assets/minimal/m4a_codec_aac.m4a"),
			(MP3, "../tests/files/assets/minimal/full_test.mp3"),
			(MPC, "../tests/files/assets/minimal/mpc_sv8.mpc"),
			(OPUS, "../tests/files/assets/minimal/full_test.opus"),
			(RIFF, "../tests/files/assets/minimal/wav_format_pcm.wav"),
			(SPEEX, "../tests/files/assets/minimal/full_test.spx"),
			(VORBIS, "../tests/files/assets/minimal/full_test.ogg"),
			(WAVPACK, "../tests/files/assets/minimal/full_test.wv"),
		]
	);
}

criterion_group!(benches, content_infer_read);
criterion_main!(benches);
