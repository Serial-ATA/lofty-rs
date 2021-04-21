use criterion::{black_box, criterion_group, criterion_main, Criterion};
use lofty::{DetermineFrom, Tag};

macro_rules! test_read {
	($function:ident, $path:expr) => {
		fn $function() {
			let _ = Tag::new()
				.read_from_path($path, DetermineFrom::Signature)
				.unwrap();
		}
	};
}

test_read!(read_ape, "tests/assets/a.ape");
test_read!(read_flac, "tests/assets/a.flac");
test_read!(read_m4a, "tests/assets/a.m4a");
test_read!(read_mp3, "tests/assets/a.mp3");
test_read!(read_ogg, "tests/assets/a.ogg");
test_read!(read_opus, "tests/assets/a.opus");
test_read!(read_wav, "tests/assets/a.wav");

fn bench_sig(c: &mut Criterion) {
	c.bench_function("APE - Signature", |b| b.iter(|| read_ape()));
	c.bench_function("FLAC - Signature", |b| b.iter(|| read_flac()));
	c.bench_function("MP4 - Signature", |b| b.iter(|| read_m4a()));
	c.bench_function("MP3 - Signature", |b| b.iter(|| read_mp3()));
	c.bench_function("OGG - Signature", |b| b.iter(|| read_ogg()));
	c.bench_function("OPUS - Signature", |b| b.iter(|| read_opus()));
	c.bench_function("WAV - Signature", |b| b.iter(|| read_wav()));
}

criterion_group!(benches, bench_sig);
criterion_main!(benches);
