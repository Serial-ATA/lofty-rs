use lofty::ape::ApeTag;
use lofty::id3::v1::ID3v1Tag;
use lofty::id3::v2::ID3v2Tag;
use lofty::iff::aiff::AIFFTextChunks;
use lofty::iff::wav::RIFFInfoList;
use lofty::mp4::Ilst;
use lofty::ogg::VorbisComments;
use lofty::{Accessor, TagExt};

use criterion::{criterion_group, criterion_main, Criterion};

macro_rules! bench_tag_write {
	($c:ident, [$(($NAME:literal, $tag:ty)),+ $(,)?]) => {
		let mut g = $c.benchmark_group("Tag writing");

		$(
			g.bench_function(
				$NAME,
				|b| b.iter(|| {
					let mut v = Vec::new();
					let mut tag = <$tag>::default();

					tag.set_artist(String::from("Foo artist"));
					tag.set_title(String::from("Bar title"));
					tag.set_album(String::from("Baz album"));
					tag.dump_to(&mut v).unwrap();
				})
			);
		)+
	}
}

fn bench_write(c: &mut Criterion) {
	bench_tag_write!(
		c,
		[
			("AIFF Text Chunks", AIFFTextChunks),
			("APEv2", ApeTag),
			("ID3v2", ID3v2Tag),
			("ID3v1", ID3v1Tag),
			("MP4 Ilst", Ilst),
			("RIFF INFO", RIFFInfoList),
			("Vorbis Comments", VorbisComments),
		]
	);
}

criterion_group!(benches, bench_write);
criterion_main!(benches);
