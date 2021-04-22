use criterion::{criterion_group, criterion_main, Criterion};
use lofty::Tag;

macro_rules! test_read {
	($function:ident, $path:expr) => {
		fn $function() {
			let _ = Tag::new().read_from_path($path).unwrap();
		}
	};
}

test_read!(read_ape, "tests/assets/a.ape");
test_read!(read_flac, "tests/assets/a.flac");
test_read!(read_m4a, "tests/assets/a.m4a");
test_read!(read_mp3, "tests/assets/a.mp3");
test_read!(read_ogg, "tests/assets/a.ogg");
test_read!(read_opus, "tests/assets/a.opus");
test_read!(read_wav, "tests/assets/a-id3.wav");

fn bench_ext(c: &mut Criterion) {
	c.bench_function("APE - Extension", |b| b.iter(|| read_ape()));
	c.bench_function("FLAC - Extension", |b| b.iter(|| read_flac()));
	c.bench_function("MP4 - Extension", |b| b.iter(|| read_m4a()));
	c.bench_function("MP3 - Extension", |b| b.iter(|| read_mp3()));
	c.bench_function("OGG - Extension", |b| b.iter(|| read_ogg()));
	c.bench_function("OPUS - Extension", |b| b.iter(|| read_opus()));
	c.bench_function("WAV - Extension", |b| b.iter(|| read_wav()));
}

criterion_group!(benches, bench_ext);
criterion_main!(benches);
