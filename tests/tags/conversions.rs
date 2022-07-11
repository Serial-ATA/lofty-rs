// Tests for special case conversions

use lofty::id3::v2::{Frame, FrameFlags, FrameValue, ID3v2Tag, LanguageFrame, TextEncoding};
use lofty::{ItemKey, Tag, TagType};

#[test]
fn tag_to_id3v2_lang_frame() {
	let mut tag = Tag::new(TagType::ID3v2);
	tag.insert_text(ItemKey::Lyrics, String::from("Test lyrics"));
	tag.insert_text(ItemKey::Comment, String::from("Test comment"));

	let id3: ID3v2Tag = tag.into();

	assert_eq!(
		id3.get("USLT"),
		Frame::new(
			"USLT",
			FrameValue::UnSyncText(LanguageFrame {
				encoding: TextEncoding::UTF8,
				language: String::from("eng"),
				description: String::new(),
				content: String::from("Test lyrics")
			}),
			FrameFlags::default()
		)
		.ok()
		.as_ref()
	);

	assert_eq!(
		id3.get("COMM"),
		Frame::new(
			"COMM",
			FrameValue::Comment(LanguageFrame {
				encoding: TextEncoding::UTF8,
				language: String::from("eng"),
				description: String::new(),
				content: String::from("Test comment")
			}),
			FrameFlags::default()
		)
		.ok()
		.as_ref()
	);
}
