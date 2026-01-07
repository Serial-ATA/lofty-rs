// Tests for special case conversions

use lofty::TextEncoding;
use lofty::id3::v2::{
	CommentFrame, Frame, FrameId, Id3v2Tag, PopularimeterFrame, UnsynchronizedTextFrame,
};
use lofty::iff::wav::RiffInfoList;
use lofty::mp4::Ilst;
use lofty::ogg::VorbisComments;
use lofty::tag::items::popularimeter::{
	Popularimeter, RatingProvider, StarRating, set_custom_rating_provider,
};
use lofty::tag::{ItemKey, Tag, TagType};

use std::borrow::Cow;

#[test_log::test]
fn tag_to_id3v2_lang_frame() {
	let mut tag = Tag::new(TagType::Id3v2);
	tag.insert_text(ItemKey::Lyrics, String::from("Test lyrics"));
	tag.insert_text(ItemKey::Comment, String::from("Test comment"));

	let id3: Id3v2Tag = tag.into();

	assert_eq!(
		id3.get(&FrameId::Valid(Cow::Borrowed("USLT"))),
		Some(&Frame::UnsynchronizedText(UnsynchronizedTextFrame::new(
			TextEncoding::UTF8,
			*b"XXX",
			String::new(),
			String::from("Test lyrics")
		)))
	);

	assert_eq!(
		id3.get(&FrameId::Valid(Cow::Borrowed("COMM"))),
		Some(&Frame::Comment(CommentFrame::new(
			TextEncoding::UTF8,
			*b"XXX",
			String::new(),
			String::from("Test comment")
		)))
	);
}

#[test_log::test]
fn popm_conversions() {
	let expected = Popularimeter::musicbee(StarRating::Three, 5);
	for tag_type in [
		TagType::Id3v2,
		TagType::VorbisComments,
		TagType::Mp4Ilst,
		TagType::RiffInfo,
	] {
		let mut tag = Tag::new(tag_type);
		tag.insert_text(ItemKey::Popularimeter, expected.to_string());

		let mut check_play_counter = true;
		match tag_type {
			TagType::Id3v2 => {
				let t: Id3v2Tag = tag.into();
				tag = t.into();
			},
			TagType::Mp4Ilst => {
				let t: Ilst = tag.into();
				tag = t.into();
			},
			TagType::VorbisComments => {
				let t: VorbisComments = tag.into();
				tag = t.into();
				// Unsupported
				check_play_counter = false;
			},
			TagType::RiffInfo => {
				let t: RiffInfoList = tag.into();
				tag = t.into();
				// Unsupported
				check_play_counter = false;
			},
			_ => unreachable!(),
		}

		let mut ratings = tag.ratings();
		let popm = ratings.next().unwrap();
		assert!(ratings.next().is_none());

		assert_eq!(popm.email(), expected.email());
		assert_eq!(popm.rating(), expected.rating());

		if check_play_counter {
			assert_eq!(popm.play_counter, expected.play_counter);
		}
	}
}

#[test_log::test]
fn popm_custom_provider() {
	let frame = PopularimeterFrame::new("foo@example.com", 128, 40);

	let mut tag = Id3v2Tag::new();
	tag.insert(Frame::Popularimeter(frame.clone()));

	// By default, "128" should map to 3 stars
	let tag: Tag = tag.into();
	let popm = tag.ratings().next().unwrap();

	assert_eq!(popm.email(), Some("foo@example.com"));
	assert_eq!(popm.rating(), StarRating::Three);
	assert_eq!(popm.play_counter, 40);

	// Now make a provider where 128 maps to one star
	struct CustomRatingProvider;

	impl RatingProvider for CustomRatingProvider {
		fn supports_email(&self, email: &str) -> bool {
			email.starts_with("foo")
		}

		fn rate(&self, tag_type: TagType, rating: StarRating) -> u8 {
			match tag_type {
				TagType::Id3v2 => match rating {
					StarRating::One => 128,
					_ => unreachable!(),
				},
				_ => unreachable!(),
			}
		}

		fn convert_raw(&self, tag_type: TagType, rating: u8) -> StarRating {
			match tag_type {
				TagType::Id3v2 => match rating {
					128 => StarRating::One,
					_ => unreachable!(),
				},
				_ => unreachable!(),
			}
		}
	}

	set_custom_rating_provider(CustomRatingProvider);

	let mut tag = Id3v2Tag::new();
	tag.insert(Frame::Popularimeter(frame.clone()));
	// Some other email that the provider doesn't support
	tag.insert(Frame::Popularimeter(PopularimeterFrame::new(
		"bar@example.com",
		128,
		40,
	)));

	let tag: Tag = tag.into();
	let mut ratings = tag.ratings();
	let popm = ratings.next().unwrap();
	assert!(ratings.next().is_none()); // The second popm should be ignored, since the email isn't supported

	assert_eq!(popm.email(), Some("foo@example.com"));
	assert_eq!(popm.rating(), StarRating::One);
	assert_eq!(popm.play_counter, 40);
}
