// Tests for special case conversions

use lofty::id3::v2::{CommentFrame, Frame, FrameFlags, FrameId, Id3v2Tag, UnsynchronizedTextFrame};
use lofty::{ItemKey, Tag, TagType, TextEncoding};
use std::borrow::Cow;

#[test]
fn tag_to_id3v2_lang_frame() {
	let mut tag = Tag::new(TagType::Id3v2);
	tag.insert_text(ItemKey::Lyrics, String::from("Test lyrics"));
	tag.insert_text(ItemKey::Comment, String::from("Test comment"));

	let id3: Id3v2Tag = tag.into();

	assert_eq!(
		id3.get(&FrameId::Valid(Cow::Borrowed("USLT"))),
		Frame::new(
			"USLT",
			UnsynchronizedTextFrame {
				encoding: TextEncoding::UTF8,
				language: *b"eng",
				description: String::new(),
				content: String::from("Test lyrics")
			},
			FrameFlags::default()
		)
		.ok()
		.as_ref()
	);

	assert_eq!(
		id3.get(&FrameId::Valid(Cow::Borrowed("COMM"))),
		Frame::new(
			"COMM",
			CommentFrame {
				encoding: TextEncoding::UTF8,
				language: *b"eng",
				description: String::new(),
				content: String::from("Test comment")
			},
			FrameFlags::default()
		)
		.ok()
		.as_ref()
	);
}
