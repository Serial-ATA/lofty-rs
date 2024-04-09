use lofty::{ParseOptions, Probe};

use iai_callgrind::{library_benchmark, library_benchmark_group, main};

use std::hint::black_box;
use std::io::Cursor;

macro_rules! test_read_file {
	([$(($NAME:ident, $path:expr)),+ $(,)?]) => {
		$(
			paste::paste! {
				#[library_benchmark]
				fn [<$NAME:lower>]() {
					const $NAME: &[u8] = include_bytes!($path);

					black_box(Probe::new(Cursor::new($NAME))
						.options(ParseOptions::new())
						.guess_file_type()
						.unwrap()
						.read()
						.unwrap());
				}
			}
		)+
	}
}

test_read_file!([
	(AAC, "../benches_assets/01 TempleOS Hymn Risen (Remix).aac"),
	(
		AIFF,
		"../benches_assets/01 TempleOS Hymn Risen (Remix).aiff"
	),
	(APE, "../benches_assets/01 TempleOS Hymn Risen (Remix).ape"),
	(
		FLAC,
		"../benches_assets/01 TempleOS Hymn Risen (Remix).flac"
	),
	(MP4, "../benches_assets/01 TempleOS Hymn Risen (Remix).m4a"),
	(MP3, "../benches_assets/01 TempleOS Hymn Risen (Remix).mp3"),
	(MPC, "../benches_assets/01 TempleOS Hymn Risen (Remix).mpc"),
	(
		OPUS,
		"../benches_assets/01 TempleOS Hymn Risen (Remix).opus"
	),
	(RIFF, "../benches_assets/01 TempleOS Hymn Risen (Remix).wav"),
	(
		SPEEX,
		"../benches_assets/01 TempleOS Hymn Risen (Remix).spx"
	),
	(
		VORBIS,
		"../benches_assets/01 TempleOS Hymn Risen (Remix).ogg"
	),
	(
		WAVPACK,
		"../benches_assets/01 TempleOS Hymn Risen (Remix).wv"
	),
]);

library_benchmark_group!(
	name = file_reading;
	benchmarks = aac, aiff, ape, flac, mp4, mp3, mpc, opus, riff, speex, vorbis, wavpack
);
main!(library_benchmark_groups = file_reading);
