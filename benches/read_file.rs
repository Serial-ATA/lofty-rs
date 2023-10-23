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
			(AAC, "assets/01 TempleOS Hymn Risen (Remix).aac"),
			(AIFF, "assets/01 TempleOS Hymn Risen (Remix).aiff"),
			(APE, "assets/01 TempleOS Hymn Risen (Remix).ape"),
			(FLAC, "assets/01 TempleOS Hymn Risen (Remix).flac"),
			(MP4, "assets/01 TempleOS Hymn Risen (Remix).m4a"),
			(MP3, "assets/01 TempleOS Hymn Risen (Remix).mp3"),
			(MPC, "assets/01 TempleOS Hymn Risen (Remix).mpc"),
			(OPUS, "assets/01 TempleOS Hymn Risen (Remix).opus"),
			(RIFF, "assets/01 TempleOS Hymn Risen (Remix).wav"),
			(SPEEX, "assets/01 TempleOS Hymn Risen (Remix).spx"),
			(VORBIS, "assets/01 TempleOS Hymn Risen (Remix).ogg"),
			(WAVPACK, "assets/01 TempleOS Hymn Risen (Remix).wv"),
		]
	);
}

criterion_group!(benches, content_infer_read);
criterion_main!(benches);
