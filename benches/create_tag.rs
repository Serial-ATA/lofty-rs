use lofty::ape::ApeTag;
use lofty::id3::v1::Id3v1Tag;
use lofty::id3::v2::Id3v2Tag;
use lofty::iff::aiff::AIFFTextChunks;
use lofty::iff::wav::RIFFInfoList;
use lofty::mp4::Ilst;
use lofty::ogg::VorbisComments;
use lofty::{Accessor, MimeType, Picture, PictureType, TagExt, WriteOptions};

use criterion::{criterion_group, criterion_main, Criterion};

const ENCODER: &str = "Lavf57.56.101";

macro_rules! bench_tag_write {
	($c:ident, [$(($NAME:literal, $tag:ty, |$tag_:ident| $extra_block:block)),+ $(,)?]) => {
		let mut g = $c.benchmark_group("Tag writing");

		$(
			g.bench_function(
				$NAME,
				|b| b.iter(|| {
					let mut v = Vec::new();
					let mut $tag_ = <$tag>::default();

					$tag_.set_artist(String::from("Dave Eddy"));
					$tag_.set_title(String::from("TempleOS Hymn Risen (Remix)"));
					$tag_.set_album(String::from("Summer"));
					$tag_.set_year(2017);
					$tag_.set_track(1);
					$tag_.set_genre(String::from("Electronic"));
					$extra_block
					$tag_.dump_to(&mut v, WriteOptions::default()).unwrap();
				})
			);
		)+
	}
}

fn bench_write(c: &mut Criterion) {
	bench_tag_write!(
		c,
		[
			("AIFF Text Chunks", AIFFTextChunks, |tag| {}),
			("APEv2", ApeTag, |tag| {
				use lofty::ape::ApeItem;
				use lofty::ItemValue;

				let picture = Picture::new_unchecked(
					PictureType::CoverFront,
					Some(MimeType::Jpeg),
					None,
					include_bytes!("../benches_assets/cover.jpg").to_vec(),
				);

				tag.insert(
					ApeItem::new(
						String::from("Cover (Front)"),
						ItemValue::Binary(picture.as_ape_bytes()),
					)
					.unwrap(),
				);
				tag.insert(
					ApeItem::new(
						String::from("Encoder"),
						ItemValue::Text(String::from(ENCODER)),
					)
					.unwrap(),
				);
			}),
			("ID3v2", Id3v2Tag, |tag| {
				use lofty::id3::v2::{Frame, FrameFlags, TextInformationFrame};
				use lofty::TextEncoding;

				let picture = Picture::new_unchecked(
					PictureType::CoverFront,
					Some(MimeType::Jpeg),
					None,
					include_bytes!("../benches_assets/cover.jpg").to_vec(),
				);

				tag.insert_picture(picture);
				tag.insert(
					Frame::new(
						"TSSE",
						TextInformationFrame {
							encoding: TextEncoding::Latin1,
							value: String::from(ENCODER),
						},
						FrameFlags::default(),
					)
					.unwrap(),
				);
			}),
			("ID3v1", Id3v1Tag, |tag| {}),
			("MP4 Ilst", Ilst, |tag| {
				use lofty::mp4::{Atom, AtomData, AtomIdent};

				let picture = Picture::new_unchecked(
					PictureType::CoverFront,
					Some(MimeType::Jpeg),
					None,
					include_bytes!("../benches_assets/cover.jpg").to_vec(),
				);

				tag.insert_picture(picture);
				tag.insert(Atom::new(
					AtomIdent::Fourcc(*b"\xa9too"),
					AtomData::UTF8(String::from(ENCODER)),
				));
			}),
			("RIFF INFO", RIFFInfoList, |tag| {
				tag.insert(String::from("ISFT"), String::from(ENCODER));
			}),
			("Vorbis Comments", VorbisComments, |tag| {
				use lofty::ogg::OggPictureStorage;

				let picture = Picture::new_unchecked(
					PictureType::CoverFront,
					Some(MimeType::Jpeg),
					None,
					include_bytes!("../benches_assets/cover.jpg").to_vec(),
				);

				let _ = tag.insert_picture(picture, None).unwrap();
				tag.push(String::from("ENCODER"), String::from(ENCODER));
			}),
		]
	);
}

criterion_group!(benches, bench_write);
criterion_main!(benches);
