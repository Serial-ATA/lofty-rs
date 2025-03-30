// Tests for special case conversions

use lofty::TextEncoding;
use lofty::id3::v2::{CommentFrame, Frame, FrameId, Id3v2Tag, UnsynchronizedTextFrame};
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
