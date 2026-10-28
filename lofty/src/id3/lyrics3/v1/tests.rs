use crate::config::WriteOptions;
use crate::id3::lyrics3::v1::Lyrics3Tag;
use crate::id3::lyrics3::v1::constants::{LYRICS3_FOOTER, LYRICS3_HEADER};
use crate::tag::{ItemKey, Tag, TagExt, TagType};

fn tag_bytes() -> Vec<u8> {
	let mut tag = Vec::new();

	tag.extend(LYRICS3_HEADER.as_bytes());
	for line in ["[00:00]Awesome lyrics\r\n", "[00:10]Written by Lofty"] {
		tag.extend(line.as_bytes());
	}
	tag.extend(LYRICS3_FOOTER.as_bytes());

	tag
}

fn expected_tag() -> Lyrics3Tag {
	Lyrics3Tag {
		lyrics: String::from("[00:00]Awesome lyrics\r\n[00:10]Written by Lofty"),
	}
}

#[test_log::test]
fn parse_lyrics3() {
	let parsed_tag = Lyrics3Tag::parse(&tag_bytes()).unwrap();
	assert_eq!(expected_tag(), parsed_tag);
}

#[test_log::test]
fn lyrics3_re_read() {
	let parsed_tag = Lyrics3Tag::parse(&tag_bytes()).unwrap();

	let mut writer = Vec::new();
	parsed_tag
		.dump_to(&mut writer, WriteOptions::default())
		.unwrap();

	let temp_parsed_tag = Lyrics3Tag::parse(&writer).unwrap();

	assert_eq!(parsed_tag, temp_parsed_tag);
}

#[test_log::test]
fn lyrics3_to_tag() {
	let lyrics3 = Lyrics3Tag::parse(&tag_bytes()).unwrap();

	let tag: Tag = lyrics3.into();
	assert_eq!(
		tag.get_string(ItemKey::Lyrics),
		Some(&*expected_tag().lyrics)
	)
}

#[test_log::test]
fn tag_to_lyrics3() {
	let mut tag = Tag::new(TagType::Lyrics3);
	tag.insert_text(ItemKey::Lyrics, expected_tag().lyrics);

	let lyrics3_tag: Lyrics3Tag = tag.into();

	assert_eq!(lyrics3_tag.lyrics, expected_tag().lyrics);
}

#[test_log::test]
fn forbidden_lyrics3_content() {
	for content in [
		"blahblahblahLYRICSBEGINblahblahblah",
		"blahblahblahLYRICSENDblahblahblah",
		"fooÿbar",
	] {
		let tag = Lyrics3Tag {
			lyrics: String::from(content),
		};

		tag.dump_to(
			&mut Vec::new(),
			WriteOptions::new().lossy_text_encoding(false),
		)
		.expect_err("should fail");
	}
}
