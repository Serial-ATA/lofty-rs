#![allow(missing_docs)]

use lofty::ape::ApeTag;
use lofty::config::WriteOptions;
use lofty::id3::v1::Id3v1Tag;
use lofty::id3::v2::{FrameId, Id3v2Tag};
use lofty::iff::aiff::AiffTextChunks;
use lofty::iff::wav::RiffInfoList;
use lofty::mp4::Ilst;
use lofty::ogg::VorbisComments;
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::tag::items::Timestamp;
use lofty::tag::{Accessor, TagExt};

use std::borrow::Cow;

use gungraun::{library_benchmark, library_benchmark_group, main};

const ENCODER: &str = "Lavf57.56.101";

macro_rules! bench_tag_write {
	([$(($NAME:ident, $tag:ty, |$tag_:ident| $extra_block:block)),+ $(,)?]) => {
		$(
			#[library_benchmark]
			fn $NAME() {
				let mut v = Vec::new();
				let mut $tag_ = <$tag>::default();

				$tag_.set_artist(String::from("Dave Eddy"));
				$tag_.set_title(String::from("TempleOS Hymn Risen (Remix)"));
				$tag_.set_album(String::from("Summer"));
				$tag_.set_date(Timestamp {
					year: 2017,
					..Timestamp::default()
				});
				$tag_.set_track(1);
				$tag_.set_genre(String::from("Electronic"));
				$extra_block;
				$tag_.dump_to(&mut v, WriteOptions::default()).unwrap();
			}
		)+
	}
}

bench_tag_write!([
	(aiff_text_chunks, AiffTextChunks, |tag| {}),
	(apev2, ApeTag, |tag| {
		use lofty::ape::ApeItem;
		use lofty::tag::ItemValue;

		let picture = Picture::unchecked(include_bytes!("./assets/cover.jpg").to_vec())
			.pic_type(PictureType::CoverFront)
			.mime_type(MimeType::Jpeg)
			.build();

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
	(id3v2, Id3v2Tag, |tag| {
		use lofty::TextEncoding;
		use lofty::id3::v2::{Frame, TextInformationFrame};

		let picture = Picture::unchecked(include_bytes!("./assets/cover.jpg").to_vec())
			.pic_type(PictureType::CoverFront)
			.mime_type(MimeType::Jpeg)
			.build();

		tag.insert_picture(picture);
		tag.insert(Frame::Text(TextInformationFrame::new(
			FrameId::Valid(Cow::Borrowed("TSSE")),
			TextEncoding::Latin1,
			String::from(ENCODER),
		)));
	}),
	(id3v1, Id3v1Tag, |tag| {}),
	(ilst, Ilst, |tag| {
		use lofty::mp4::{Atom, AtomData, AtomIdent};

		let picture = Picture::unchecked(include_bytes!("./assets/cover.jpg").to_vec())
			.pic_type(PictureType::CoverFront)
			.mime_type(MimeType::Jpeg)
			.build();

		tag.insert_picture(picture);
		tag.insert(Atom::new(
			AtomIdent::Fourcc(*b"\xa9too"),
			AtomData::UTF8(String::from(ENCODER)),
		));
	}),
	(riff_info, RiffInfoList, |tag| {
		tag.insert(String::from("ISFT"), String::from(ENCODER));
	}),
	(vorbis_comments, VorbisComments, |tag| {
		use lofty::ogg::OggPictureStorage;

		let picture = Picture::unchecked(include_bytes!("./assets/cover.jpg").to_vec())
			.pic_type(PictureType::CoverFront)
			.mime_type(MimeType::Jpeg)
			.build();

		let _ = tag.insert_picture(picture, None).unwrap();
		tag.push(String::from("ENCODER"), String::from(ENCODER));
	}),
]);

library_benchmark_group!(
	name = tag_writing;
	benchmarks = aiff_text_chunks, apev2, id3v2, id3v1, ilst, riff_info, vorbis_comments
);
main!(library_benchmark_groups = tag_writing);
