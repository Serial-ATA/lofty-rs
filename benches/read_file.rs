#![allow(missing_docs)]

use lofty::config::ParseOptions;
use lofty::probe::Probe;

use gungraun::{library_benchmark, library_benchmark_group, main};

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
	(AAC, "./assets/01 TempleOS Hymn Risen (Remix).aac"),
	(AIFF, "./assets/01 TempleOS Hymn Risen (Remix).aiff"),
	(APE, "./assets/01 TempleOS Hymn Risen (Remix).ape"),
	(FLAC, "./assets/01 TempleOS Hymn Risen (Remix).flac"),
	(MP4, "./assets/01 TempleOS Hymn Risen (Remix).m4a"),
	(MP3, "./assets/01 TempleOS Hymn Risen (Remix).mp3"),
	(MPC, "./assets/01 TempleOS Hymn Risen (Remix).mpc"),
	(OPUS, "./assets/01 TempleOS Hymn Risen (Remix).opus"),
	(RIFF, "./assets/01 TempleOS Hymn Risen (Remix).wav"),
	(SPEEX, "./assets/01 TempleOS Hymn Risen (Remix).spx"),
	(VORBIS, "./assets/01 TempleOS Hymn Risen (Remix).ogg"),
	(WAVPACK, "./assets/01 TempleOS Hymn Risen (Remix).wv"),
]);

library_benchmark_group!(
	name = file_reading;
	benchmarks = aac, aiff, ape, flac, mp4, mp3, mpc, opus, riff, speex, vorbis, wavpack
);
main!(library_benchmark_groups = file_reading);
