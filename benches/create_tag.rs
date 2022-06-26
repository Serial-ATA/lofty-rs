use lofty::ape::ApeTag;
use lofty::id3::v1::ID3v1Tag;
use lofty::id3::v2::ID3v2Tag;
use lofty::iff::{AIFFTextChunks, RiffInfoList};
use lofty::mp4::Ilst;
use lofty::ogg::VorbisComments;
use lofty::{Accessor, TagExt};

use criterion::{criterion_group, criterion_main, Criterion};

macro_rules! bench_tag_write {
	($function:ident, $tag:ty) => {
		fn $function() {
			let mut v = Vec::new();
			let mut tag = <$tag>::default();

			tag.set_artist(String::from("Foo artist"));
			tag.set_title(String::from("Bar title"));
			tag.set_album(String::from("Baz album"));
			tag.dump_to(&mut v).unwrap();
		}
	};
}

bench_tag_write!(aiff_text, AIFFTextChunks);
bench_tag_write!(ape, ApeTag);
bench_tag_write!(id3v2, ID3v2Tag);
bench_tag_write!(id3v1, ID3v1Tag);
bench_tag_write!(ilst, Ilst);
bench_tag_write!(riff_info, RiffInfoList);
bench_tag_write!(vorbis_comments, VorbisComments);

fn bench_write(c: &mut Criterion) {
	let mut g = c.benchmark_group("Tag writing");
	g.bench_function("AIFF Text Chunks", |b| b.iter(aiff_text));
	g.bench_function("APEv2", |b| b.iter(ape));
	g.bench_function("ID3v2", |b| b.iter(id3v2));
	g.bench_function("ID3v1", |b| b.iter(id3v1));
	g.bench_function("MP4 Ilst", |b| b.iter(ilst));
	g.bench_function("RIFF INFO", |b| b.iter(riff_info));
	g.bench_function("Vorbis Comments", |b| b.iter(vorbis_comments));
}

criterion_group!(benches, bench_write);
criterion_main!(benches);
