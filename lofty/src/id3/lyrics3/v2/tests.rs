use crate::config::{GlobalOptions, ParseOptions, ParsingMode, WriteOptions};
use crate::id3::lyrics3::v2::constants::{
	BASIC_TEXT_FIELD_MAX_SIZE, FIELD_MAX_SIZE, LYRICS3V2_FOOTER, LYRICS3V2_HEADER, PATH_MAX,
};
use crate::id3::lyrics3::v2::{Image, Lyrics3v2Tag, Lyrics3v2TagFlags};
use crate::tag::items::Timestamp;
use crate::tag::{ItemKey, Tag, TagExt, TagType};

fn tag_bytes() -> Vec<u8> {
	let mut tag = Vec::new();

	tag.extend(LYRICS3V2_HEADER.as_bytes());

	// Indications
	tag.extend(b"IND00003");
	tag.extend(b"110");

	// Extended album name
	tag.extend(b"EAL00011");
	tag.extend(b"Lofty Stuff");

	// Extended artist name
	tag.extend(b"EAR00014");
	tag.extend(b"Foo & the Bars");

	// Extended artist name
	tag.extend(b"ETT00007");
	tag.extend(b"BazLand");

	// Info
	tag.extend(b"INF00088");
	tag.extend(b"This track has lots of interesting info.\r\nSo much in fact, that it spans multiple lines!");

	// Author
	tag.extend(b"AUT00010");
	tag.extend(b"Serial-ATA");

	// Lyrics
	tag.extend(b"LYR00046");
	tag.extend(b"[00:00]Awesome lyrics\r\n[00:10]Written by Lofty");

	// Images
	tag.extend(b"IMG00094");
	tag.extend(b"foo.jpg||foo is a great picture||[15:01]\r\n");
	tag.extend(b"no_description.jpg||||[15:01]\r\n");
	tag.extend(b"filename_only.jpg||||");

	tag.extend(b"000348");
	tag.extend(LYRICS3V2_FOOTER.as_bytes());

	tag
}

fn expected_tag_concrete() -> Lyrics3v2Tag {
	Lyrics3v2Tag {
		flags: Lyrics3v2TagFlags {
			synchronized_lyrics: true,
			random_selection: false,
		},
		lyrics: Some(String::from(
			"[00:00]Awesome lyrics\r\n[00:10]Written by Lofty",
		)),
		info: Some(String::from(
			"This track has lots of interesting info.\r\nSo much in fact, that it spans multiple \
			 lines!",
		)),
		author: Some(String::from("Serial-ATA")),
		extended_album_name: Some(String::from("Lofty Stuff")),
		extended_artist_name: Some(String::from("Foo & the Bars")),
		extended_track_title: Some(String::from("BazLand")),
		images: vec![
			Image {
				filename: String::from("foo.jpg"),
				description: Some(String::from("foo is a great picture")),
				timestamp: Some(Timestamp {
					minute: Some(15),
					second: Some(1),
					..Timestamp::default()
				}),
			},
			Image {
				filename: String::from("no_description.jpg"),
				description: None,
				timestamp: Some(Timestamp {
					minute: Some(15),
					second: Some(1),
					..Timestamp::default()
				}),
			},
			Image {
				filename: String::from("filename_only.jpg"),
				description: None,
				timestamp: None,
			},
		],
	}
}

fn expected_tag() -> Tag {
	let mut tag = Tag::new(TagType::Lyrics3v2);
	tag.insert_text(
		ItemKey::Lyrics,
		String::from("[00:00]Awesome lyrics\r\n[00:10]Written by Lofty"),
	);
	tag.insert_text(
		ItemKey::Comment,
		String::from(
			"This track has lots of interesting info.\r\nSo much in fact, that it spans multiple \
			 lines!",
		),
	);
	tag.insert_text(ItemKey::Lyricist, String::from("Serial-ATA"));
	tag.insert_text(ItemKey::AlbumTitle, String::from("Lofty Stuff"));
	tag.insert_text(ItemKey::TrackArtist, String::from("Foo & the Bars"));
	tag.insert_text(ItemKey::TrackTitle, String::from("BazLand"));

	tag
}

#[test_log::test]
fn parse_lyrics3v2() {
	let parsed_tag = Lyrics3v2Tag::parse(
		&tag_bytes(),
		ParseOptions::new().parsing_mode(ParsingMode::Strict),
	)
	.unwrap();
	assert_eq!(expected_tag_concrete(), parsed_tag);
}

#[test_log::test]
fn lyrics3v2_re_read() {
	let parsed_tag = Lyrics3v2Tag::parse(
		&tag_bytes(),
		ParseOptions::new().parsing_mode(ParsingMode::Strict),
	)
	.unwrap();

	let mut writer = Vec::new();
	parsed_tag
		.dump_to(&mut writer, WriteOptions::default())
		.unwrap();

	let temp_parsed_tag = Lyrics3v2Tag::parse(
		&writer,
		ParseOptions::new().parsing_mode(ParsingMode::Strict),
	)
	.unwrap();

	assert_eq!(parsed_tag, temp_parsed_tag);
}

#[test_log::test]
fn lyrics3v2_to_tag() {
	let lyrics3 = Lyrics3v2Tag::parse(
		&tag_bytes(),
		ParseOptions::new().parsing_mode(ParsingMode::Strict),
	)
	.unwrap();

	let tag: Tag = lyrics3.into();
	assert_eq!(tag.items.len(), expected_tag().items.len());
	assert!(expected_tag().items.iter().all(|i| tag.items.contains(i)));
}

#[test_log::test]
fn tag_to_lyrics3v2() {
	let tag = expected_tag();
	let lyrics3_tag: Lyrics3v2Tag = tag.into();

	let mut expected_lyrics3v2 = expected_tag_concrete();
	expected_lyrics3v2.images = Vec::new();
	expected_lyrics3v2.flags = Lyrics3v2TagFlags::default();

	assert_eq!(lyrics3_tag, expected_lyrics3v2);
}

#[test_log::test]
fn special_items_roundtrip() {
	let expected_lyrics3v2 = expected_tag_concrete();

	// Should preserve images and flags in the companion tag
	let tag: Tag = expected_lyrics3v2.clone().into();
	let roundtripped: Lyrics3v2Tag = tag.into();

	assert_eq!(expected_lyrics3v2, roundtripped);
}

#[test_log::test]
fn forbidden_lyrics3v2_content() {
	let tag = Lyrics3v2Tag {
		lyrics: Some(String::from("fooÿbar")),
		..Lyrics3v2Tag::default()
	};

	tag.dump_to(
		&mut Vec::new(),
		WriteOptions::new().lossy_text_encoding(false),
	)
	.expect_err("should fail");
}

#[test_log::test]
fn lyrics3v2_fields_truncate() {
	std::thread::scope(|scope| {
		scope.spawn(|| {
			crate::config::apply_global_options(GlobalOptions::new().allocation_limit(100_000 * 2));

			let tag = Lyrics3v2Tag {
				lyrics: Some(String::from("W").repeat((FIELD_MAX_SIZE as usize) + 1)),
				author: Some(String::from("X").repeat((BASIC_TEXT_FIELD_MAX_SIZE as usize) + 1)),
				images: vec![Image {
					filename: String::from("Y").repeat((PATH_MAX as usize) + 1),
					description: Some(
						String::from("Z").repeat((BASIC_TEXT_FIELD_MAX_SIZE as usize) + 1),
					),
					timestamp: None,
				}],
				..Lyrics3v2Tag::default()
			};

			let mut writer = Vec::new();
			tag.dump_to(&mut writer, WriteOptions::new()).unwrap();

			let parsed_tag = Lyrics3v2Tag::parse(
				&writer,
				ParseOptions::new().parsing_mode(ParsingMode::Strict),
			)
			.unwrap();
			assert_eq!(
				parsed_tag.lyrics,
				Some(String::from("W").repeat(FIELD_MAX_SIZE as usize)),
				"should be truncated by 1 byte"
			);
			assert_eq!(
				parsed_tag.author,
				Some(String::from("X").repeat(BASIC_TEXT_FIELD_MAX_SIZE as usize)),
				"should be truncated by 1 byte"
			);
			assert_eq!(
				parsed_tag.images[0],
				Image {
					filename: String::from("Y").repeat(PATH_MAX as usize),
					description: Some(String::from("Z").repeat(BASIC_TEXT_FIELD_MAX_SIZE as usize)),
					timestamp: None,
				}
			)
		});
	})
}

#[test_log::test]
fn ignore_unknown_lyrics3v2_fields() {
	let mut tag = Vec::new();

	tag.extend(LYRICS3V2_HEADER.as_bytes());

	// Indications
	tag.extend(b"IND00003");
	tag.extend(b"000");

	// Extended album name
	tag.extend(b"EAL00011");
	tag.extend(b"Lofty Stuff");

	// Random unknown field
	tag.extend(b"HUH00020");
	tag.extend(b"Some unknown content");

	tag.extend(b"000069");
	tag.extend(LYRICS3V2_FOOTER.as_bytes());

	let lyrics3v2 =
		Lyrics3v2Tag::parse(&tag, ParseOptions::new().parsing_mode(ParsingMode::Strict)).unwrap();

	assert_eq!(
		lyrics3v2.extended_album_name.as_deref(),
		Some("Lofty Stuff")
	)
}
