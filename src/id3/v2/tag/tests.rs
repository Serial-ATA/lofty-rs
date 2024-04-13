use crate::config::ParsingMode;
use crate::id3::v2::header::Id3v2Header;
use crate::id3::v2::items::Popularimeter;
use crate::id3::v2::util::pairs::DEFAULT_NUMBER_IN_PAIR;
use crate::picture::MimeType;
use crate::tag::utils::test_utils::read_path;

use super::*;

fn read_tag(path: &str) -> Id3v2Tag {
	let tag_bytes = read_path(path);

	let mut reader = Cursor::new(&tag_bytes[..]);

	let header = Id3v2Header::parse(&mut reader).unwrap();
	crate::id3::v2::read::parse_id3v2(&mut reader, header, ParsingMode::Strict).unwrap()
}

#[test]
fn parse_id3v2() {
	let mut expected_tag = Id3v2Tag::default();

	let encoding = TextEncoding::Latin1;
	let flags = FrameFlags::default();

	expected_tag.insert(
		Frame::new(
			"TPE1",
			FrameValue::Text(TextInformationFrame {
				encoding,
				value: String::from("Bar artist"),
			}),
			flags,
		)
		.unwrap(),
	);

	expected_tag.insert(
		Frame::new(
			"TIT2",
			FrameValue::Text(TextInformationFrame {
				encoding,
				value: String::from("Foo title"),
			}),
			flags,
		)
		.unwrap(),
	);

	expected_tag.insert(
		Frame::new(
			"TALB",
			FrameValue::Text(TextInformationFrame {
				encoding,
				value: String::from("Baz album"),
			}),
			flags,
		)
		.unwrap(),
	);

	expected_tag.insert(
		Frame::new(
			COMMENT_FRAME_ID,
			FrameValue::Comment(CommentFrame {
				encoding,
				language: *b"eng",
				description: EMPTY_CONTENT_DESCRIPTOR,
				content: String::from("Qux comment"),
			}),
			flags,
		)
		.unwrap(),
	);

	expected_tag.insert(
		Frame::new(
			"TDRC",
			FrameValue::Text(TextInformationFrame {
				encoding,
				value: String::from("1984"),
			}),
			flags,
		)
		.unwrap(),
	);

	expected_tag.insert(
		Frame::new(
			"TRCK",
			FrameValue::Text(TextInformationFrame {
				encoding,
				value: String::from("1"),
			}),
			flags,
		)
		.unwrap(),
	);

	expected_tag.insert(
		Frame::new(
			"TCON",
			FrameValue::Text(TextInformationFrame {
				encoding,
				value: String::from("Classical"),
			}),
			flags,
		)
		.unwrap(),
	);

	let parsed_tag = read_tag("tests/tags/assets/id3v2/test.id3v24");

	assert_eq!(expected_tag, parsed_tag);
}

#[test]
fn id3v2_re_read() {
	let parsed_tag = read_tag("tests/tags/assets/id3v2/test.id3v24");

	let mut writer = Vec::new();
	parsed_tag
		.dump_to(&mut writer, WriteOptions::default())
		.unwrap();

	let temp_reader = &mut &*writer;

	let temp_header = Id3v2Header::parse(temp_reader).unwrap();
	let temp_parsed_tag =
		crate::id3::v2::read::parse_id3v2(temp_reader, temp_header, ParsingMode::Strict).unwrap();

	assert_eq!(parsed_tag, temp_parsed_tag);
}

#[test]
fn id3v2_to_tag() {
	let id3v2 = read_tag("tests/tags/assets/id3v2/test.id3v24");

	let tag: Tag = id3v2.into();

	crate::tag::utils::test_utils::verify_tag(&tag, true, true);
}

#[test]
fn id3v2_to_tag_popm() {
	let id3v2 = read_tag("tests/tags/assets/id3v2/test_popm.id3v24");

	let tag: Tag = id3v2.into();

	assert_eq!(
		tag.get_binary(&ItemKey::Popularimeter, false),
		Some(
			&[
				b'f', b'o', b'o', b'@', b'b', b'a', b'r', b'.', b'c', b'o', b'm', 0, 196, 0, 0,
				255, 255,
			][..]
		),
	);
}

#[test]
fn tag_to_id3v2_popm() {
	let mut tag = Tag::new(TagType::Id3v2);
	tag.insert(TagItem::new(
		ItemKey::Popularimeter,
		ItemValue::Binary(vec![
			b'f', b'o', b'o', b'@', b'b', b'a', b'r', b'.', b'c', b'o', b'm', 0, 196, 0, 0, 255,
			255,
		]),
	));

	let expected = Popularimeter {
		email: String::from("foo@bar.com"),
		rating: 196,
		counter: 65535,
	};

	let converted_tag: Id3v2Tag = tag.into();

	assert_eq!(converted_tag.frames.len(), 1);
	let actual_frame = converted_tag.frames.first().unwrap();

	assert_eq!(actual_frame.id, FrameId::Valid(Cow::Borrowed("POPM")));
	// Note: as POPM frames are considered equal by email alone, each field must
	// be separately validated
	match actual_frame.content() {
		FrameValue::Popularimeter(pop) => {
			assert_eq!(pop.email, expected.email);
			assert_eq!(pop.rating, expected.rating);
			assert_eq!(pop.counter, expected.counter);
		},
		_ => unreachable!(),
	}
}

#[test]
fn fail_write_bad_frame() {
	let mut tag = Id3v2Tag::default();
	tag.insert(Frame {
		id: FrameId::Valid(Cow::Borrowed("ABCD")),
		value: FrameValue::Url(UrlLinkFrame(String::from("FOO URL"))),
		flags: FrameFlags::default(),
	});

	let res = tag.dump_to(&mut Vec::<u8>::new(), WriteOptions::default());

	assert!(res.is_err());
	assert_eq!(
		res.unwrap_err().to_string(),
		String::from("ID3v2: Attempted to write an invalid frame. ID: \"ABCD\", Value: \"Url\"")
	);
}

#[test]
fn tag_to_id3v2() {
	fn verify_frame(tag: &Id3v2Tag, id: &str, value: &str) {
		let frame = tag.get(&FrameId::Valid(Cow::Borrowed(id)));

		assert!(frame.is_some());

		let frame = frame.unwrap();

		assert_eq!(
			frame.content(),
			&FrameValue::Text(TextInformationFrame {
				encoding: TextEncoding::UTF8,
				value: String::from(value)
			})
		);
	}

	let tag = crate::tag::utils::test_utils::create_tag(TagType::Id3v2);

	let id3v2_tag: Id3v2Tag = tag.into();

	verify_frame(&id3v2_tag, "TIT2", "Foo title");
	verify_frame(&id3v2_tag, "TPE1", "Bar artist");
	verify_frame(&id3v2_tag, "TALB", "Baz album");

	let frame = id3v2_tag
		.get(&FrameId::Valid(Cow::Borrowed(COMMENT_FRAME_ID)))
		.unwrap();
	assert_eq!(
		frame.content(),
		&FrameValue::Comment(CommentFrame {
			encoding: TextEncoding::Latin1,
			language: *b"eng",
			description: EMPTY_CONTENT_DESCRIPTOR,
			content: String::from("Qux comment")
		})
	);

	verify_frame(&id3v2_tag, "TRCK", "1");
	verify_frame(&id3v2_tag, "TCON", "Classical");
}

#[allow(clippy::field_reassign_with_default)]
fn create_full_test_tag(version: Id3v2Version) -> Id3v2Tag {
	let mut tag = Id3v2Tag::default();
	tag.original_version = version;

	let encoding = TextEncoding::UTF16;
	let flags = FrameFlags::default();

	tag.insert(Frame {
		id: FrameId::Valid(Cow::Borrowed("TIT2")),
		value: FrameValue::Text(TextInformationFrame {
			encoding,
			value: String::from("TempleOS Hymn Risen (Remix)"),
		}),
		flags,
	});

	tag.insert(Frame {
		id: FrameId::Valid(Cow::Borrowed("TPE1")),
		value: FrameValue::Text(TextInformationFrame {
			encoding,
			value: String::from("Dave Eddy"),
		}),
		flags,
	});

	tag.insert(Frame {
		id: FrameId::Valid(Cow::Borrowed("TRCK")),
		value: FrameValue::Text(TextInformationFrame {
			encoding: TextEncoding::Latin1,
			value: String::from("1"),
		}),
		flags,
	});

	tag.insert(Frame {
		id: FrameId::Valid(Cow::Borrowed("TALB")),
		value: FrameValue::Text(TextInformationFrame {
			encoding,
			value: String::from("Summer"),
		}),
		flags,
	});

	tag.insert(Frame {
		id: FrameId::Valid(Cow::Borrowed("TDRC")),
		value: FrameValue::Text(TextInformationFrame {
			encoding,
			value: String::from("2017"),
		}),
		flags,
	});

	tag.insert(Frame {
		id: FrameId::Valid(Cow::Borrowed("TCON")),
		value: FrameValue::Text(TextInformationFrame {
			encoding,
			value: String::from("Electronic"),
		}),
		flags,
	});

	tag.insert(Frame {
		id: FrameId::Valid(Cow::Borrowed("TLEN")),
		value: FrameValue::Text(TextInformationFrame {
			encoding: TextEncoding::UTF16,
			value: String::from("213017"),
		}),
		flags,
	});

	tag.insert(Frame {
		id: FrameId::Valid(Cow::Borrowed("APIC")),
		value: FrameValue::Picture(AttachedPictureFrame {
			encoding: TextEncoding::Latin1,
			picture: Picture {
				pic_type: PictureType::CoverFront,
				mime_type: Some(MimeType::Png),
				description: None,
				data: read_path("tests/tags/assets/id3v2/test_full_cover.png").into(),
			},
		}),
		flags,
	});

	tag
}

#[test]
fn id3v24_full() {
	let tag = create_full_test_tag(Id3v2Version::V4);
	let parsed_tag = read_tag("tests/tags/assets/id3v2/test_full.id3v24");

	assert_eq!(tag, parsed_tag);
}

#[test]
fn id3v23_full() {
	let tag = create_full_test_tag(Id3v2Version::V3);
	let parsed_tag = read_tag("tests/tags/assets/id3v2/test_full.id3v23");

	assert_eq!(tag, parsed_tag);
}

#[test]
fn id3v22_full() {
	let tag = create_full_test_tag(Id3v2Version::V2);
	let parsed_tag = read_tag("tests/tags/assets/id3v2/test_full.id3v22");

	assert_eq!(tag, parsed_tag);
}

#[test]
fn id3v24_footer() {
	let mut tag = create_full_test_tag(Id3v2Version::V4);
	tag.flags.footer = true;

	let mut writer = Vec::new();
	tag.dump_to(&mut writer, WriteOptions::default()).unwrap();

	let mut reader = &mut &writer[..];

	let header = Id3v2Header::parse(&mut reader).unwrap();
	let _ = crate::id3::v2::read::parse_id3v2(reader, header, ParsingMode::Strict).unwrap();

	assert_eq!(writer[3..10], writer[writer.len() - 7..])
}

#[test]
fn issue_36() {
	let picture_data = vec![0; 200];

	let picture = Picture::new_unchecked(
		PictureType::CoverFront,
		Some(MimeType::Jpeg),
		Some(String::from("cover")),
		picture_data,
	);

	let mut tag = Tag::new(TagType::Id3v2);
	tag.push_picture(picture.clone());

	let mut writer = Vec::new();
	tag.dump_to(&mut writer, WriteOptions::default()).unwrap();

	let mut reader = &mut &writer[..];

	let header = Id3v2Header::parse(&mut reader).unwrap();
	let tag = crate::id3::v2::read::parse_id3v2(reader, header, ParsingMode::Strict).unwrap();

	assert_eq!(tag.len(), 1);
	assert_eq!(
		tag.frames.first(),
		Some(&Frame {
			id: FrameId::Valid(Cow::Borrowed("APIC")),
			value: FrameValue::Picture(AttachedPictureFrame {
				encoding: TextEncoding::UTF8,
				picture
			}),
			flags: FrameFlags::default()
		})
	);
}

#[test]
fn popm_frame() {
	let parsed_tag = read_tag("tests/tags/assets/id3v2/test_popm.id3v24");

	assert_eq!(parsed_tag.frames.len(), 1);
	let popm_frame = parsed_tag.frames.first().unwrap();

	assert_eq!(popm_frame.id, FrameId::Valid(Cow::Borrowed("POPM")));
	assert_eq!(
		popm_frame.value,
		FrameValue::Popularimeter(Popularimeter {
			email: String::from("foo@bar.com"),
			rating: 196,
			counter: 65535
		})
	)
}

#[test]
fn multi_value_frame_to_tag() {
	use crate::traits::Accessor;
	let mut tag = Id3v2Tag::default();

	tag.set_artist(String::from("foo\0bar\0baz"));

	let tag: Tag = tag.into();
	let collected_artists = tag.get_strings(&ItemKey::TrackArtist).collect::<Vec<_>>();
	assert_eq!(&collected_artists, &["foo", "bar", "baz"])
}

#[test]
fn multi_item_tag_to_id3v2() {
	use crate::traits::Accessor;
	let mut tag = Tag::new(TagType::Id3v2);

	tag.push_unchecked(TagItem::new(
		ItemKey::TrackArtist,
		ItemValue::Text(String::from("foo")),
	));
	tag.push_unchecked(TagItem::new(
		ItemKey::TrackArtist,
		ItemValue::Text(String::from("bar")),
	));
	tag.push_unchecked(TagItem::new(
		ItemKey::TrackArtist,
		ItemValue::Text(String::from("baz")),
	));

	let tag: Id3v2Tag = tag.into();
	assert_eq!(tag.artist().as_deref(), Some("foo/bar/baz"))
}

#[test]
fn utf16_txxx_with_single_bom() {
	let _ = read_tag("tests/tags/assets/id3v2/issue_53.id3v24");
}

#[test]
fn replaygain_tag_conversion() {
	let mut tag = Id3v2Tag::default();
	tag.insert(
		Frame::new(
			"TXXX",
			FrameValue::UserText(ExtendedTextFrame {
				encoding: TextEncoding::UTF8,
				description: String::from("REPLAYGAIN_ALBUM_GAIN"),
				content: String::from("-10.43 dB"),
			}),
			FrameFlags::default(),
		)
		.unwrap(),
	);

	let tag: Tag = tag.into();

	assert_eq!(tag.item_count(), 1);
	assert_eq!(
		tag.items[0],
		TagItem {
			item_key: ItemKey::ReplayGainAlbumGain,
			item_value: ItemValue::Text(String::from("-10.43 dB"))
		}
	);
}

#[test]
fn multi_value_roundtrip() {
	let mut tag = Tag::new(TagType::Id3v2);
	// 1st: Multi-valued text frames
	tag.insert_text(ItemKey::TrackArtist, "TrackArtist 1".to_owned());
	tag.push(TagItem::new(
		ItemKey::TrackArtist,
		ItemValue::Text("TrackArtist 2".to_owned()),
	));
	tag.insert_text(ItemKey::AlbumArtist, "AlbumArtist 1".to_owned());
	tag.push(TagItem::new(
		ItemKey::AlbumArtist,
		ItemValue::Text("AlbumArtist 2".to_owned()),
	));
	tag.insert_text(ItemKey::TrackTitle, "TrackTitle 1".to_owned());
	tag.push(TagItem::new(
		ItemKey::TrackTitle,
		ItemValue::Text("TrackTitle 2".to_owned()),
	));
	tag.insert_text(ItemKey::AlbumTitle, "AlbumTitle 1".to_owned());
	tag.push(TagItem::new(
		ItemKey::AlbumTitle,
		ItemValue::Text("AlbumTitle 2".to_owned()),
	));
	tag.insert_text(ItemKey::ContentGroup, "ContentGroup 1".to_owned());
	tag.push(TagItem::new(
		ItemKey::ContentGroup,
		ItemValue::Text("ContentGroup 2".to_owned()),
	));
	tag.insert_text(ItemKey::Genre, "Genre 1".to_owned());
	tag.push(TagItem::new(
		ItemKey::Genre,
		ItemValue::Text("Genre 2".to_owned()),
	));
	tag.insert_text(ItemKey::Mood, "Mood 1".to_owned());
	tag.push(TagItem::new(
		ItemKey::Mood,
		ItemValue::Text("Mood 2".to_owned()),
	));
	tag.insert_text(ItemKey::Composer, "Composer 1".to_owned());
	tag.push(TagItem::new(
		ItemKey::Composer,
		ItemValue::Text("Composer 2".to_owned()),
	));
	tag.insert_text(ItemKey::Conductor, "Conductor 1".to_owned());
	tag.push(TagItem::new(
		ItemKey::Conductor,
		ItemValue::Text("Conductor 2".to_owned()),
	));
	// 2nd: Multi-valued language frames
	tag.insert_text(ItemKey::Comment, "Comment 1".to_owned());
	tag.push(TagItem::new(
		ItemKey::Comment,
		ItemValue::Text("Comment 2".to_owned()),
	));
	assert_eq!(20, tag.len());

	let id3v2 = Id3v2Tag::from(tag.clone());
	let (split_remainder, split_tag) = id3v2.split_tag();

	assert_eq!(0, split_remainder.0.len());
	assert_eq!(tag.len(), split_tag.len());
	// The ordering of items/frames matters, see above!
	// TODO: Replace with an unordered comparison.
	assert_eq!(tag.items, split_tag.items);
}

#[test]
fn comments() {
	let mut tag = Id3v2Tag::default();
	let encoding = TextEncoding::Latin1;
	let flags = FrameFlags::default();
	let custom_descriptor = "lofty-rs";

	assert!(tag.comment().is_none());

	// Add an empty comment (which is a valid use case).
	tag.set_comment(String::new());
	assert_eq!(Some(Cow::Borrowed("")), tag.comment());

	// Insert a custom comment frame
	assert!(tag
		.frames
		.iter()
		.find_map(|frame| filter_comment_frame_by_description(frame, custom_descriptor))
		.is_none());
	tag.insert(
		Frame::new(
			COMMENT_FRAME_ID,
			FrameValue::Comment(CommentFrame {
				encoding,
				language: *b"eng",
				description: custom_descriptor.to_owned(),
				content: String::from("Qux comment"),
			}),
			flags,
		)
		.unwrap(),
	);
	// Verify that the regular comment still exists
	assert_eq!(Some(Cow::Borrowed("")), tag.comment());
	assert_eq!(1, tag.comments().count());

	tag.remove_comment();
	assert!(tag.comment().is_none());

	// Verify that the comment with the custom descriptor still exists
	assert!(tag
		.frames
		.iter()
		.find_map(|frame| filter_comment_frame_by_description(frame, custom_descriptor))
		.is_some());
}

#[test]
fn txxx_wxxx_tag_conversion() {
	let txxx_frame = Frame::new(
		"TXXX",
		FrameValue::UserText(ExtendedTextFrame {
			encoding: TextEncoding::UTF8,
			description: String::from("FOO_TEXT_FRAME"),
			content: String::from("foo content"),
		}),
		FrameFlags::default(),
	)
	.unwrap();

	let wxxx_frame = Frame::new(
		"WXXX",
		FrameValue::UserUrl(ExtendedUrlFrame {
			encoding: TextEncoding::UTF8,
			description: String::from("BAR_URL_FRAME"),
			content: String::from("bar url"),
		}),
		FrameFlags::default(),
	)
	.unwrap();

	let mut tag = Id3v2Tag::default();

	tag.insert(txxx_frame.clone());
	tag.insert(wxxx_frame.clone());

	let tag: Tag = tag.into();

	assert_eq!(tag.item_count(), 2);
	let expected_items = [
		TagItem::new(
			ItemKey::Unknown(String::from("FOO_TEXT_FRAME")),
			ItemValue::Text(String::from("foo content")),
		),
		TagItem::new(
			ItemKey::Unknown(String::from("BAR_URL_FRAME")),
			ItemValue::Locator(String::from("bar url")),
		),
	];
	assert!(expected_items
		.iter()
		.zip(tag.items())
		.all(|(expected, actual)| expected == actual));

	let tag: Id3v2Tag = tag.into();

	assert_eq!(tag.frames.len(), 2);
	assert_eq!(&tag.frames, &[txxx_frame, wxxx_frame])
}

#[test]
fn user_defined_frames_conversion() {
	let mut id3v2 = Id3v2Tag::default();
	id3v2.insert(
		Frame::new(
			"TXXX",
			FrameValue::UserText(ExtendedTextFrame {
				encoding: TextEncoding::UTF8,
				description: String::from("FOO_BAR"),
				content: String::from("foo content"),
			}),
			FrameFlags::default(),
		)
		.unwrap(),
	);

	let (split_remainder, split_tag) = id3v2.split_tag();
	assert_eq!(split_remainder.0.len(), 0);
	assert_eq!(split_tag.len(), 1);

	let id3v2 = split_remainder.merge_tag(split_tag);

	// Verify we properly convert user defined frames between Tag <-> ID3v2Tag round trips
	assert_eq!(
		id3v2.frames.first(),
		Some(&Frame {
			id: FrameId::Valid(Cow::Borrowed("TXXX")),
			value: FrameValue::UserText(ExtendedTextFrame {
				description: String::from("FOO_BAR"),
				encoding: TextEncoding::UTF8, // Not considered by PartialEq!
				content: String::new(),       // Not considered by PartialEq!
			}),
			flags: FrameFlags::default(),
		})
	);

	// Verify we properly convert user defined frames when writing a Tag, which has to convert
	// to the reference types.
	let (_remainder, tag) = id3v2.clone().split_tag();
	assert_eq!(tag.len(), 1);

	let mut content = Vec::new();
	tag.dump_to(&mut content, WriteOptions::default()).unwrap();
	assert!(!content.is_empty());

	// And verify we can reread the tag
	let mut reader = std::io::Cursor::new(&content[..]);

	let header = Id3v2Header::parse(&mut reader).unwrap();
	let reparsed =
		crate::id3::v2::read::parse_id3v2(&mut reader, header, ParsingMode::Strict).unwrap();

	assert_eq!(id3v2, reparsed);
}

#[test]
fn set_track() {
	let mut id3v2 = Id3v2Tag::default();
	let track = 1;

	id3v2.set_track(track);

	assert_eq!(id3v2.track().unwrap(), track);
	assert!(id3v2.track_total().is_none());
}

#[test]
fn set_track_total() {
	let mut id3v2 = Id3v2Tag::default();
	let track_total = 2;

	id3v2.set_track_total(track_total);

	assert_eq!(id3v2.track().unwrap(), DEFAULT_NUMBER_IN_PAIR);
	assert_eq!(id3v2.track_total().unwrap(), track_total);
}

#[test]
fn set_track_and_track_total() {
	let mut id3v2 = Id3v2Tag::default();
	let track = 1;
	let track_total = 2;

	id3v2.set_track(track);
	id3v2.set_track_total(track_total);

	assert_eq!(id3v2.track().unwrap(), track);
	assert_eq!(id3v2.track_total().unwrap(), track_total);
}

#[test]
fn set_track_total_and_track() {
	let mut id3v2 = Id3v2Tag::default();
	let track_total = 2;
	let track = 1;

	id3v2.set_track_total(track_total);
	id3v2.set_track(track);

	assert_eq!(id3v2.track_total().unwrap(), track_total);
	assert_eq!(id3v2.track().unwrap(), track);
}

#[test]
fn set_disk() {
	let mut id3v2 = Id3v2Tag::default();
	let disk = 1;

	id3v2.set_disk(disk);

	assert_eq!(id3v2.disk().unwrap(), disk);
	assert!(id3v2.disk_total().is_none());
}

#[test]
fn set_disk_total() {
	let mut id3v2 = Id3v2Tag::default();
	let disk_total = 2;

	id3v2.set_disk_total(disk_total);

	assert_eq!(id3v2.disk().unwrap(), DEFAULT_NUMBER_IN_PAIR);
	assert_eq!(id3v2.disk_total().unwrap(), disk_total);
}

#[test]
fn set_disk_and_disk_total() {
	let mut id3v2 = Id3v2Tag::default();
	let disk = 1;
	let disk_total = 2;

	id3v2.set_disk(disk);
	id3v2.set_disk_total(disk_total);

	assert_eq!(id3v2.disk().unwrap(), disk);
	assert_eq!(id3v2.disk_total().unwrap(), disk_total);
}

#[test]
fn set_disk_total_and_disk() {
	let mut id3v2 = Id3v2Tag::default();
	let disk_total = 2;
	let disk = 1;

	id3v2.set_disk_total(disk_total);
	id3v2.set_disk(disk);

	assert_eq!(id3v2.disk_total().unwrap(), disk_total);
	assert_eq!(id3v2.disk().unwrap(), disk);
}

#[test]
fn track_number_tag_to_id3v2() {
	use crate::traits::Accessor;
	let track_number = 1;

	let mut tag = Tag::new(TagType::Id3v2);

	tag.push(TagItem::new(
		ItemKey::TrackNumber,
		ItemValue::Text(track_number.to_string()),
	));

	let tag: Id3v2Tag = tag.into();

	assert_eq!(tag.track().unwrap(), track_number);
	assert!(tag.track_total().is_none());
}

#[test]
fn track_total_tag_to_id3v2() {
	use crate::traits::Accessor;
	let track_total = 2;

	let mut tag = Tag::new(TagType::Id3v2);

	tag.push(TagItem::new(
		ItemKey::TrackTotal,
		ItemValue::Text(track_total.to_string()),
	));

	let tag: Id3v2Tag = tag.into();

	assert_eq!(tag.track().unwrap(), DEFAULT_NUMBER_IN_PAIR);
	assert_eq!(tag.track_total().unwrap(), track_total);
}

#[test]
fn track_number_and_track_total_tag_to_id3v2() {
	use crate::traits::Accessor;
	let track_number = 1;
	let track_total = 2;

	let mut tag = Tag::new(TagType::Id3v2);

	tag.push(TagItem::new(
		ItemKey::TrackNumber,
		ItemValue::Text(track_number.to_string()),
	));

	tag.push(TagItem::new(
		ItemKey::TrackTotal,
		ItemValue::Text(track_total.to_string()),
	));

	let tag: Id3v2Tag = tag.into();

	assert_eq!(tag.track().unwrap(), track_number);
	assert_eq!(tag.track_total().unwrap(), track_total);
}

#[test]
fn disk_number_tag_to_id3v2() {
	use crate::traits::Accessor;
	let disk_number = 1;

	let mut tag = Tag::new(TagType::Id3v2);

	tag.push(TagItem::new(
		ItemKey::DiscNumber,
		ItemValue::Text(disk_number.to_string()),
	));

	let tag: Id3v2Tag = tag.into();

	assert_eq!(tag.disk().unwrap(), disk_number);
	assert!(tag.disk_total().is_none());
}

#[test]
fn disk_total_tag_to_id3v2() {
	use crate::traits::Accessor;
	let disk_total = 2;

	let mut tag = Tag::new(TagType::Id3v2);

	tag.push(TagItem::new(
		ItemKey::DiscTotal,
		ItemValue::Text(disk_total.to_string()),
	));

	let tag: Id3v2Tag = tag.into();

	assert_eq!(tag.disk().unwrap(), DEFAULT_NUMBER_IN_PAIR);
	assert_eq!(tag.disk_total().unwrap(), disk_total);
}

#[test]
fn disk_number_and_disk_total_tag_to_id3v2() {
	use crate::traits::Accessor;
	let disk_number = 1;
	let disk_total = 2;

	let mut tag = Tag::new(TagType::Id3v2);

	tag.push(TagItem::new(
		ItemKey::DiscNumber,
		ItemValue::Text(disk_number.to_string()),
	));

	tag.push(TagItem::new(
		ItemKey::DiscTotal,
		ItemValue::Text(disk_total.to_string()),
	));

	let tag: Id3v2Tag = tag.into();

	assert_eq!(tag.disk().unwrap(), disk_number);
	assert_eq!(tag.disk_total().unwrap(), disk_total);
}

fn create_tag_with_trck_and_tpos_frame(content: &'static str) -> Tag {
	fn insert_frame(id: &'static str, content: &'static str, tag: &mut Id3v2Tag) {
		tag.insert(new_text_frame(
			FrameId::Valid(Cow::Borrowed(id)),
			content.to_string(),
			FrameFlags::default(),
		));
	}

	let mut tag = Id3v2Tag::default();

	insert_frame("TRCK", content, &mut tag);
	insert_frame("TPOS", content, &mut tag);

	tag.into()
}

#[test]
fn valid_trck_and_tpos_frame() {
	fn assert_valid(content: &'static str, number: Option<u32>, total: Option<u32>) {
		let tag = create_tag_with_trck_and_tpos_frame(content);

		assert_eq!(tag.track(), number);
		assert_eq!(tag.track_total(), total);
		assert_eq!(tag.disk(), number);
		assert_eq!(tag.disk_total(), total);
	}

	assert_valid("0", Some(0), None);
	assert_valid("1", Some(1), None);
	assert_valid("0/0", Some(0), Some(0));
	assert_valid("1/2", Some(1), Some(2));
	assert_valid("010/011", Some(10), Some(11));
	assert_valid(" 1/2 ", Some(1), Some(2));
	assert_valid("1 / 2", Some(1), Some(2));
}

#[test]
fn invalid_trck_and_tpos_frame() {
	fn assert_invalid(content: &'static str) {
		let tag = create_tag_with_trck_and_tpos_frame(content);

		assert!(tag.track().is_none());
		assert!(tag.track_total().is_none());
		assert!(tag.disk().is_none());
		assert!(tag.disk_total().is_none());
	}

	assert_invalid("");
	assert_invalid(" ");
	assert_invalid("/");
	assert_invalid("/1");
	assert_invalid("1/");
	assert_invalid("a/b");
	assert_invalid("1/2/3");
	assert_invalid("1//2");
	assert_invalid("0x1/0x2");
}

#[test]
fn ufid_frame_with_musicbrainz_record_id() {
	let mut id3v2 = Id3v2Tag::default();
	let unknown_ufid_frame = UniqueFileIdentifierFrame {
		owner: "other".to_owned(),
		identifier: b"0123456789".to_vec(),
	};
	id3v2.insert(
		Frame::new(
			"UFID",
			FrameValue::UniqueFileIdentifier(unknown_ufid_frame.clone()),
			FrameFlags::default(),
		)
		.unwrap(),
	);
	let musicbrainz_recording_id = b"189002e7-3285-4e2e-92a3-7f6c30d407a2";
	let musicbrainz_recording_id_frame = UniqueFileIdentifierFrame {
		owner: MUSICBRAINZ_UFID_OWNER.to_owned(),
		identifier: musicbrainz_recording_id.to_vec(),
	};
	id3v2.insert(
		Frame::new(
			"UFID",
			FrameValue::UniqueFileIdentifier(musicbrainz_recording_id_frame.clone()),
			FrameFlags::default(),
		)
		.unwrap(),
	);
	assert_eq!(2, id3v2.len());

	let (split_remainder, split_tag) = id3v2.split_tag();
	assert_eq!(split_remainder.0.len(), 1);
	assert_eq!(split_tag.len(), 1);
	assert_eq!(
		ItemValue::Text(String::from_utf8(musicbrainz_recording_id.to_vec()).unwrap()),
		*split_tag
			.get_items(&ItemKey::MusicBrainzRecordingId)
			.next()
			.unwrap()
			.value()
	);

	let id3v2 = split_remainder.merge_tag(split_tag);
	assert_eq!(2, id3v2.len());
	match &id3v2.frames[..] {
		[Frame {
			id: _,
			value:
				FrameValue::UniqueFileIdentifier(UniqueFileIdentifierFrame {
					owner: first_owner,
					identifier: first_identifier,
				}),
			flags: _,
		}, Frame {
			id: _,
			value:
				FrameValue::UniqueFileIdentifier(UniqueFileIdentifierFrame {
					owner: second_owner,
					identifier: second_identifier,
				}),
			flags: _,
		}] => {
			assert_eq!(&unknown_ufid_frame.owner, first_owner);
			assert_eq!(&unknown_ufid_frame.identifier, first_identifier);
			assert_eq!(&musicbrainz_recording_id_frame.owner, second_owner);
			assert_eq!(
				&musicbrainz_recording_id_frame.identifier,
				second_identifier
			);
		},
		_ => unreachable!(),
	}
}

#[test]
fn get_set_user_defined_text() {
	let description = String::from("FOO_BAR");
	let content = String::from("Baz!\0Qux!");
	let description2 = String::from("FOO_BAR_2");
	let content2 = String::new();

	let mut id3v2 = Id3v2Tag::default();
	let txxx_frame = Frame::new(
		"TXXX",
		ExtendedTextFrame {
			encoding: TextEncoding::UTF8,
			description: description.clone(),
			content: content.clone(),
		},
		FrameFlags::default(),
	)
	.unwrap();

	id3v2.insert(txxx_frame.clone());

	// Insert another to verify we can search through multiple
	let txxx_frame2 = Frame::new(
		"TXXX",
		ExtendedTextFrame {
			encoding: TextEncoding::UTF8,
			description: description2.clone(),
			content: content2.clone(),
		},
		FrameFlags::default(),
	)
	.unwrap();
	id3v2.insert(txxx_frame2);

	// We cannot get user defined texts through `get_text`
	assert!(id3v2
		.get_text(&FrameId::Valid(Cow::Borrowed("TXXX")))
		.is_none());

	assert_eq!(id3v2.get_user_text(description.as_str()), Some(&*content));

	// Wipe the tag
	id3v2.clear();

	// Same thing process as above, using simplified setter
	assert!(id3v2
		.insert_user_text(description.clone(), content.clone())
		.is_none());
	assert!(id3v2
		.insert_user_text(description2.clone(), content2.clone())
		.is_none());
	assert_eq!(id3v2.get_user_text(description.as_str()), Some(&*content));

	// Remove one frame
	assert!(id3v2.remove_user_text(&description).is_some());
	assert!(!id3v2.is_empty());

	// Now clear the remaining item
	assert!(id3v2.remove_user_text(&description2).is_some());
	assert!(id3v2.is_empty());
}

#[test]
fn read_multiple_composers_should_not_fail_with_bad_frame_length() {
	// Issue #255
	let tag = read_tag("tests/tags/assets/id3v2/multiple_composers.id3v24");
	let mut composers = tag
		.get_texts(&FrameId::Valid(Cow::Borrowed("TCOM")))
		.unwrap();

	assert_eq!(composers.next(), Some("A"));
	assert_eq!(composers.next(), Some("B"));
	assert_eq!(composers.next(), None)
}

#[test]
fn trim_end_nulls_when_reading_frame_content() {
	// Issue #273
	// Tag written by mid3v2. All frames contain null-terminated UTF-8 text
	let tag = read_tag("tests/tags/assets/id3v2/trailing_nulls.id3v24");

	// Verify that each different frame type no longer has null terminator
	let artist = tag.get_text(&FrameId::Valid(Cow::Borrowed("TPE1")));
	assert_eq!(artist.unwrap(), "Artist");

	let writer = tag.get_user_text("Writer");
	assert_eq!(writer.unwrap(), "Writer");

	let lyrics = &tag.unsync_text().next().unwrap().content;
	assert_eq!(lyrics, "Lyrics to the song");

	let comment = tag.comment().unwrap();
	assert_eq!(comment, "Comment");

	let url_frame = tag.get(&FrameId::Valid(Cow::Borrowed("WXXX"))).unwrap();
	let FrameValue::UserUrl(url) = &url_frame.value else {
		panic!("Expected a UserUrl")
	};
	assert_eq!(url.content, "https://www.myfanpage.com");
}

fn id3v2_tag_with_genre(value: &str) -> Id3v2Tag {
	let mut tag = Id3v2Tag::default();
	let frame = new_text_frame(GENRE_ID, String::from(value), FrameFlags::default());
	tag.insert(frame);
	tag
}

#[test]
fn genre_text() {
	let tag = id3v2_tag_with_genre("Dream Pop");
	assert_eq!(tag.genre(), Some(Cow::Borrowed("Dream Pop")));
}
#[test]
fn genre_id_brackets() {
	let tag = id3v2_tag_with_genre("(21)");
	assert_eq!(tag.genre(), Some(Cow::Borrowed("Ska")));
}

#[test]
fn genre_id_numeric() {
	let tag = id3v2_tag_with_genre("21");
	assert_eq!(tag.genre(), Some(Cow::Borrowed("Ska")));
}

#[test]
fn genre_id_multiple_joined() {
	let tag = id3v2_tag_with_genre("(51)(39)");
	assert_eq!(
		tag.genre(),
		Some(Cow::Borrowed("Techno-Industrial / Noise"))
	);
}

#[test]
fn genres_id_multiple() {
	let tag = id3v2_tag_with_genre("(51)(39)");
	let mut genres = tag.genres().unwrap();
	assert_eq!(genres.next(), Some("Techno-Industrial"));
	assert_eq!(genres.next(), Some("Noise"));
	assert_eq!(genres.next(), None);
}

#[test]
fn genres_id_multiple_into_tag() {
	let id3v2 = id3v2_tag_with_genre("(51)(39)");
	let tag: Tag = id3v2.into();
	let mut genres = tag.get_strings(&ItemKey::Genre);
	assert_eq!(genres.next(), Some("Techno-Industrial"));
	assert_eq!(genres.next(), Some("Noise"));
	assert_eq!(genres.next(), None);
}

#[test]
fn genres_null_separated() {
	let tag = id3v2_tag_with_genre("Samba-rock\0MPB\0Funk");
	let mut genres = tag.genres().unwrap();
	assert_eq!(genres.next(), Some("Samba-rock"));
	assert_eq!(genres.next(), Some("MPB"));
	assert_eq!(genres.next(), Some("Funk"));
	assert_eq!(genres.next(), None);
}

#[test]
fn genres_id_textual_refinement() {
	let tag = id3v2_tag_with_genre("(4)Eurodisco");
	let mut genres = tag.genres().unwrap();
	assert_eq!(genres.next(), Some("Disco"));
	assert_eq!(genres.next(), Some("Eurodisco"));
	assert_eq!(genres.next(), None);
}

#[test]
fn genres_id_bracketed_refinement() {
	let tag = id3v2_tag_with_genre("(26)(55)((I think...)");
	let mut genres = tag.genres().unwrap();
	assert_eq!(genres.next(), Some("Ambient"));
	assert_eq!(genres.next(), Some("Dream"));
	assert_eq!(genres.next(), Some("(I think...)"));
	assert_eq!(genres.next(), None);
}

#[test]
fn genres_id_remix_cover() {
	let tag = id3v2_tag_with_genre("(0)(RX)(CR)");
	let mut genres = tag.genres().unwrap();
	assert_eq!(genres.next(), Some("Blues"));
	assert_eq!(genres.next(), Some("Remix"));
	assert_eq!(genres.next(), Some("Cover"));
	assert_eq!(genres.next(), None);
}

#[test]
fn tipl_round_trip() {
	let mut tag = Id3v2Tag::default();
	let mut tipl = KeyValueFrame {
		encoding: TextEncoding::UTF8,
		key_value_pairs: Vec::new(),
	};

	// Add all supported keys
	for (_, key) in TIPL_MAPPINGS {
		tipl.key_value_pairs
			.push((String::from(*key), String::from("Serial-ATA")));
	}

	// Add one unsupported key
	tipl.key_value_pairs
		.push((String::from("Foo"), String::from("Bar")));

	tag.insert(
		Frame::new(
			"TIPL",
			FrameValue::KeyValue(tipl.clone()),
			FrameFlags::default(),
		)
		.unwrap(),
	);

	let (split_remainder, split_tag) = tag.split_tag();
	assert_eq!(split_remainder.0.len(), 1); // "Foo" is not supported
	assert_eq!(split_tag.len(), TIPL_MAPPINGS.len()); // All supported keys are present

	for (item_key, _) in TIPL_MAPPINGS {
		assert_eq!(
			split_tag
				.get(item_key)
				.map(TagItem::value)
				.and_then(ItemValue::text),
			Some("Serial-ATA")
		);
	}

	let mut id3v2 = split_remainder.merge_tag(split_tag);
	assert_eq!(id3v2.frames.len(), 1);
	match &mut id3v2.frames[..] {
		[Frame {
			id: _,
			value: FrameValue::KeyValue(tipl2),
			flags: _,
		}] => {
			// Order will not be the same, so we have to sort first
			tipl.key_value_pairs.sort();
			tipl2.key_value_pairs.sort();
			assert_eq!(tipl, *tipl2);
		},
		_ => unreachable!(),
	}
}

#[test]
fn flag_item_conversion() {
	let mut tag = Tag::new(TagType::Id3v2);
	tag.insert_text(ItemKey::FlagCompilation, "1".to_owned());
	tag.insert_text(ItemKey::FlagPodcast, "0".to_owned());

	let tag: Id3v2Tag = tag.into();
	assert_eq!(
		tag.get_text(&FrameId::Valid(Cow::Borrowed("TCMP"))),
		Some("1")
	);
	assert_eq!(
		tag.get_text(&FrameId::Valid(Cow::Borrowed("PCST"))),
		Some("0")
	);
}
