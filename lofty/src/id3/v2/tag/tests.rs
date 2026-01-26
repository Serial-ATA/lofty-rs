use crate::config::{ParseOptions, ParsingMode};
use crate::id3::v2::header::Id3v2Header;
use crate::id3::v2::items::PopularimeterFrame;
use crate::id3::v2::util::pairs::DEFAULT_NUMBER_IN_PAIR;
use crate::id3::v2::{
	ChannelInformation, ChannelType, RelativeVolumeAdjustmentFrame, TimestampFrame,
};
use crate::picture::MimeType;
use crate::tag::items::popularimeter::{Popularimeter, StarRating};
use crate::tag::items::{ENGLISH, Timestamp};
use crate::tag::utils::test_utils::read_path;

use super::*;

use std::collections::HashMap;

const COMMENT_FRAME_ID: &str = "COMM";

fn read_tag(path: &str) -> Id3v2Tag {
	let tag_bytes = read_path(path);
	read_tag_with_options(
		&tag_bytes,
		ParseOptions::new().parsing_mode(ParsingMode::Strict),
	)
}

fn read_tag_with_options(bytes: &[u8], parse_options: ParseOptions) -> Id3v2Tag {
	let mut reader = Cursor::new(bytes);

	let header = Id3v2Header::parse(&mut reader).unwrap();
	crate::id3::v2::read::parse_id3v2(&mut reader, header, parse_options).unwrap()
}

fn dump_and_re_read(tag: &Id3v2Tag, write_options: WriteOptions) -> Id3v2Tag {
	let mut tag_bytes = Vec::new();
	let mut writer = Cursor::new(&mut tag_bytes);
	tag.dump_to(&mut writer, write_options).unwrap();

	read_tag_with_options(
		&tag_bytes[..],
		ParseOptions::new().parsing_mode(ParsingMode::Strict),
	)
}

#[test_log::test]
fn parse_id3v2() {
	let mut expected_tag = Id3v2Tag::default();

	let encoding = TextEncoding::Latin1;

	expected_tag.insert(Frame::Text(TextInformationFrame::new(
		FrameId::Valid(Cow::Borrowed("TPE1")),
		encoding,
		String::from("Bar artist"),
	)));

	expected_tag.insert(Frame::Text(TextInformationFrame::new(
		FrameId::Valid(Cow::Borrowed("TIT2")),
		encoding,
		String::from("Foo title"),
	)));

	expected_tag.insert(Frame::Text(TextInformationFrame::new(
		FrameId::Valid(Cow::Borrowed("TALB")),
		encoding,
		String::from("Baz album"),
	)));

	expected_tag.insert(Frame::Comment(CommentFrame::new(
		encoding,
		*b"eng",
		EMPTY_CONTENT_DESCRIPTOR,
		String::from("Qux comment"),
	)));

	expected_tag.insert(Frame::Timestamp(TimestampFrame::new(
		FrameId::Valid(Cow::Borrowed("TDRC")),
		encoding,
		Timestamp {
			year: 1984,
			..Timestamp::default()
		},
	)));

	expected_tag.insert(Frame::Text(TextInformationFrame::new(
		FrameId::Valid(Cow::Borrowed("TRCK")),
		encoding,
		String::from("1"),
	)));

	expected_tag.insert(Frame::Text(TextInformationFrame::new(
		FrameId::Valid(Cow::Borrowed("TCON")),
		encoding,
		String::from("Classical"),
	)));

	let parsed_tag = read_tag("tests/tags/assets/id3v2/test.id3v24");

	assert_eq!(expected_tag, parsed_tag);
}

#[test_log::test]
fn id3v2_re_read() {
	let parsed_tag = read_tag("tests/tags/assets/id3v2/test.id3v24");

	let mut writer = Vec::new();
	parsed_tag
		.dump_to(&mut writer, WriteOptions::default())
		.unwrap();

	let temp_reader = &mut &*writer;

	let temp_header = Id3v2Header::parse(temp_reader).unwrap();
	let temp_parsed_tag = crate::id3::v2::read::parse_id3v2(
		temp_reader,
		temp_header,
		ParseOptions::new().parsing_mode(ParsingMode::Strict),
	)
	.unwrap();

	assert_eq!(parsed_tag, temp_parsed_tag);
}

#[test_log::test]
fn id3v2_to_tag() {
	let id3v2 = read_tag("tests/tags/assets/id3v2/test.id3v24");

	let tag: Tag = id3v2.into();

	crate::tag::utils::test_utils::verify_tag(&tag, true, true);
}

#[test_log::test]
fn fail_write_bad_frame() {
	let mut tag = Id3v2Tag::default();

	tag.insert(Frame::Url(UrlLinkFrame::new(
		FrameId::Valid(Cow::Borrowed("ABCD")),
		String::from("FOO URL"),
	)));

	let res = tag.dump_to(&mut Vec::<u8>::new(), WriteOptions::default());

	assert!(res.is_err());
	assert_eq!(
		res.unwrap_err().to_string(),
		String::from("ID3v2: Attempted to write an invalid frame. ID: \"ABCD\", Value: \"Url\"")
	);
}

#[test_log::test]
fn tag_to_id3v2() {
	fn verify_frame(tag: &Id3v2Tag, id: &str, value: &str) {
		let frame = tag.get(&FrameId::Valid(Cow::Borrowed(id)));

		assert!(frame.is_some());

		let frame = frame.unwrap();

		assert_eq!(
			frame,
			&Frame::Text(TextInformationFrame::new(
				FrameId::Valid(Cow::Borrowed(id)),
				TextEncoding::UTF8,
				String::from(value)
			)),
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
		frame,
		&Frame::Comment(CommentFrame::new(
			TextEncoding::UTF8,
			*b"XXX",
			EMPTY_CONTENT_DESCRIPTOR,
			String::from("Qux comment"),
		))
	);

	verify_frame(&id3v2_tag, "TRCK", "1");
	verify_frame(&id3v2_tag, "TCON", "Classical");
}

#[allow(clippy::field_reassign_with_default)]
fn create_full_test_tag(version: Id3v2Version) -> Id3v2Tag {
	let mut tag = Id3v2Tag::default();
	tag.original_version = version;

	let encoding = TextEncoding::UTF16;

	tag.insert(Frame::Text(TextInformationFrame::new(
		FrameId::Valid(Cow::Borrowed("TIT2")),
		encoding,
		String::from("TempleOS Hymn Risen (Remix)"),
	)));

	tag.insert(Frame::Text(TextInformationFrame::new(
		FrameId::Valid(Cow::Borrowed("TPE1")),
		encoding,
		String::from("Dave Eddy"),
	)));

	tag.insert(Frame::Text(TextInformationFrame::new(
		FrameId::Valid(Cow::Borrowed("TRCK")),
		encoding,
		String::from("1"),
	)));

	tag.insert(Frame::Text(TextInformationFrame::new(
		FrameId::Valid(Cow::Borrowed("TALB")),
		encoding,
		String::from("Summer"),
	)));

	tag.insert(Frame::Timestamp(TimestampFrame::new(
		FrameId::Valid(Cow::Borrowed("TDRC")),
		encoding,
		Timestamp {
			year: 2017,
			..Timestamp::default()
		},
	)));

	tag.insert(Frame::Text(TextInformationFrame::new(
		FrameId::Valid(Cow::Borrowed("TCON")),
		encoding,
		String::from("Electronic"),
	)));

	tag.insert(Frame::Text(TextInformationFrame::new(
		FrameId::Valid(Cow::Borrowed("TLEN")),
		encoding,
		String::from("213017"),
	)));

	tag.insert(Frame::Picture(AttachedPictureFrame::new(
		TextEncoding::Latin1,
		Picture {
			pic_type: PictureType::CoverFront,
			mime_type: Some(MimeType::Png),
			description: None,
			data: read_path("tests/tags/assets/id3v2/test_full_cover.png").into(),
		},
	)));

	tag
}

#[test_log::test]
fn id3v24_full() {
	let tag = create_full_test_tag(Id3v2Version::V4);
	let parsed_tag = read_tag("tests/tags/assets/id3v2/test_full.id3v24");

	assert_eq!(tag, parsed_tag);
}

#[test_log::test]
fn id3v23_full() {
	let mut tag = create_full_test_tag(Id3v2Version::V3);
	let mut parsed_tag = read_tag("tests/tags/assets/id3v2/test_full.id3v23");

	// Tags may change order after being read, due to the TDRC conversion
	tag.frames.sort_by_key(|frame| frame.id_str().to_string());
	parsed_tag
		.frames
		.sort_by_key(|frame| frame.id_str().to_string());
	assert_eq!(tag, parsed_tag);
}

#[test_log::test]
fn id3v22_full() {
	let tag = create_full_test_tag(Id3v2Version::V2);
	let parsed_tag = read_tag("tests/tags/assets/id3v2/test_full.id3v22");

	assert_eq!(tag, parsed_tag);
}

#[test_log::test]
fn id3v24_footer() {
	let mut tag = create_full_test_tag(Id3v2Version::V4);
	tag.flags.footer = true;

	let mut writer = Vec::new();
	tag.dump_to(&mut writer, WriteOptions::default()).unwrap();

	let mut reader = &mut &writer[..];

	let header = Id3v2Header::parse(&mut reader).unwrap();
	let _ = crate::id3::v2::read::parse_id3v2(
		reader,
		header,
		ParseOptions::new().parsing_mode(ParsingMode::Strict),
	)
	.unwrap();

	assert_eq!(writer[3..10], writer[writer.len() - 7..])
}

#[test_log::test]
fn issue_36() {
	let picture_data = vec![0; 200];

	let picture = Picture::unchecked(picture_data)
		.pic_type(PictureType::CoverFront)
		.mime_type(MimeType::Jpeg)
		.description("cover")
		.build();

	let mut tag = Tag::new(TagType::Id3v2);
	tag.push_picture(picture.clone());

	let mut writer = Vec::new();
	tag.dump_to(&mut writer, WriteOptions::default()).unwrap();

	let mut reader = &mut &writer[..];

	let header = Id3v2Header::parse(&mut reader).unwrap();
	let tag = crate::id3::v2::read::parse_id3v2(
		reader,
		header,
		ParseOptions::new().parsing_mode(ParsingMode::Strict),
	)
	.unwrap();

	assert_eq!(tag.len(), 1);
	assert_eq!(
		tag.frames.first(),
		Some(&Frame::Picture(AttachedPictureFrame::new(
			TextEncoding::UTF8,
			picture
		)))
	);
}

#[test_log::test]
fn popm_frame() {
	let parsed_tag = read_tag("tests/tags/assets/id3v2/test_popm.id3v24");

	assert_eq!(parsed_tag.frames.len(), 1);
	let popm_frame = parsed_tag.frames.first().unwrap();

	assert_eq!(popm_frame.id(), &FrameId::Valid(Cow::Borrowed("POPM")));
	assert_eq!(
		popm_frame,
		&Frame::Popularimeter(PopularimeterFrame::new(
			String::from("foo@bar.com"),
			196,
			65535
		))
	)
}

#[test_log::test]
fn multi_value_frame_to_tag() {
	let mut tag = Id3v2Tag::default();

	tag.set_artist(String::from("foo\0bar\0baz"));

	let tag: Tag = tag.into();
	let collected_artists = tag.get_strings(ItemKey::TrackArtist).collect::<Vec<_>>();
	assert_eq!(&collected_artists, &["foo", "bar", "baz"])
}

#[test_log::test]
fn multi_item_tag_to_id3v2() {
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

#[test_log::test]
fn utf16_txxx_with_single_bom() {
	let _ = read_tag("tests/tags/assets/id3v2/issue_53.id3v24");
}

#[test_log::test]
fn replaygain_tag_conversion() {
	let mut tag = Id3v2Tag::default();
	tag.insert(Frame::UserText(ExtendedTextFrame::new(
		TextEncoding::UTF8,
		String::from("REPLAYGAIN_ALBUM_GAIN"),
		String::from("-10.43 dB"),
	)));

	let tag: Tag = tag.into();

	assert_eq!(tag.item_count(), 1);
	assert_eq!(
		tag.items[0],
		TagItem::new(
			ItemKey::ReplayGainAlbumGain,
			ItemValue::Text(String::from("-10.43 dB"))
		)
	);
}

#[test_log::test]
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
	let (split_remainder, mut split_tag) = id3v2.split_tag();

	assert_eq!(0, split_remainder.0.len());
	assert_eq!(tag.len(), split_tag.len());

	for item in tag.items {
		let Some(pos) = split_tag
			.items
			.iter()
			.position(|item_split| item == *item_split)
		else {
			panic!("mismatch");
		};

		split_tag.items.remove(pos);
	}
}

#[test_log::test]
fn comments() {
	let mut tag = Id3v2Tag::default();
	let encoding = TextEncoding::Latin1;
	let custom_descriptor = "lofty-rs";

	assert!(tag.comment().is_none());

	// Add an empty comment (which is a valid use case).
	tag.set_comment(String::new());
	assert_eq!(Some(Cow::Borrowed("")), tag.comment());

	// Insert a custom comment frame
	assert!(
		tag.frames
			.iter()
			.find_map(|frame| filter_comment_frame_by_description(frame, custom_descriptor))
			.is_none()
	);
	tag.insert(Frame::Comment(CommentFrame::new(
		encoding,
		*b"eng",
		custom_descriptor.to_owned(),
		String::from("Qux comment"),
	)));
	// Verify that the regular comment still exists
	assert_eq!(Some(Cow::Borrowed("")), tag.comment());
	assert_eq!(1, tag.comments().count());

	tag.remove_comment();
	assert!(tag.comment().is_none());

	// Verify that the comment with the custom descriptor still exists
	assert!(
		tag.frames
			.iter()
			.find_map(|frame| filter_comment_frame_by_description(frame, custom_descriptor))
			.is_some()
	);
}

#[test_log::test]
fn set_track() {
	let mut id3v2 = Id3v2Tag::default();
	let track = 1;

	id3v2.set_track(track);

	assert_eq!(id3v2.track().unwrap(), track);
	assert!(id3v2.track_total().is_none());
}

#[test_log::test]
fn set_track_total() {
	let mut id3v2 = Id3v2Tag::default();
	let track_total = 2;

	id3v2.set_track_total(track_total);

	assert_eq!(id3v2.track().unwrap(), DEFAULT_NUMBER_IN_PAIR);
	assert_eq!(id3v2.track_total().unwrap(), track_total);
}

#[test_log::test]
fn set_track_and_track_total() {
	let mut id3v2 = Id3v2Tag::default();
	let track = 1;
	let track_total = 2;

	id3v2.set_track(track);
	id3v2.set_track_total(track_total);

	assert_eq!(id3v2.track().unwrap(), track);
	assert_eq!(id3v2.track_total().unwrap(), track_total);
}

#[test_log::test]
fn set_track_total_and_track() {
	let mut id3v2 = Id3v2Tag::default();
	let track_total = 2;
	let track = 1;

	id3v2.set_track_total(track_total);
	id3v2.set_track(track);

	assert_eq!(id3v2.track_total().unwrap(), track_total);
	assert_eq!(id3v2.track().unwrap(), track);
}

#[test_log::test]
fn set_disk() {
	let mut id3v2 = Id3v2Tag::default();
	let disk = 1;

	id3v2.set_disk(disk);

	assert_eq!(id3v2.disk().unwrap(), disk);
	assert!(id3v2.disk_total().is_none());
}

#[test_log::test]
fn set_disk_total() {
	let mut id3v2 = Id3v2Tag::default();
	let disk_total = 2;

	id3v2.set_disk_total(disk_total);

	assert_eq!(id3v2.disk().unwrap(), DEFAULT_NUMBER_IN_PAIR);
	assert_eq!(id3v2.disk_total().unwrap(), disk_total);
}

#[test_log::test]
fn set_disk_and_disk_total() {
	let mut id3v2 = Id3v2Tag::default();
	let disk = 1;
	let disk_total = 2;

	id3v2.set_disk(disk);
	id3v2.set_disk_total(disk_total);

	assert_eq!(id3v2.disk().unwrap(), disk);
	assert_eq!(id3v2.disk_total().unwrap(), disk_total);
}

#[test_log::test]
fn set_disk_total_and_disk() {
	let mut id3v2 = Id3v2Tag::default();
	let disk_total = 2;
	let disk = 1;

	id3v2.set_disk_total(disk_total);
	id3v2.set_disk(disk);

	assert_eq!(id3v2.disk_total().unwrap(), disk_total);
	assert_eq!(id3v2.disk().unwrap(), disk);
}

#[test_log::test]
fn track_number_tag_to_id3v2() {
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

#[test_log::test]
fn track_total_tag_to_id3v2() {
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

#[test_log::test]
fn track_number_and_track_total_tag_to_id3v2() {
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

#[test_log::test]
fn disk_number_tag_to_id3v2() {
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

#[test_log::test]
fn disk_total_tag_to_id3v2() {
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

#[test_log::test]
fn disk_number_and_disk_total_tag_to_id3v2() {
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
		));
	}

	let mut tag = Id3v2Tag::default();

	insert_frame("TRCK", content, &mut tag);
	insert_frame("TPOS", content, &mut tag);

	tag.into()
}

#[test_log::test]
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

#[test_log::test]
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

#[test_log::test]
fn ufid_frame_with_musicbrainz_record_id() {
	let mut id3v2 = Id3v2Tag::default();
	let unknown_ufid_frame =
		UniqueFileIdentifierFrame::new("other".to_owned(), b"0123456789".to_vec());
	id3v2.insert(Frame::UniqueFileIdentifier(unknown_ufid_frame.clone()));
	let musicbrainz_recording_id = b"189002e7-3285-4e2e-92a3-7f6c30d407a2";
	let musicbrainz_recording_id_frame = UniqueFileIdentifierFrame::new(
		MUSICBRAINZ_UFID_OWNER.to_owned(),
		musicbrainz_recording_id.to_vec(),
	);
	id3v2.insert(Frame::UniqueFileIdentifier(
		musicbrainz_recording_id_frame.clone(),
	));
	assert_eq!(2, id3v2.len());

	let (split_remainder, split_tag) = id3v2.split_tag();
	assert_eq!(split_remainder.0.len(), 1);
	assert_eq!(split_tag.len(), 1);
	assert_eq!(
		ItemValue::Text(String::from_utf8(musicbrainz_recording_id.to_vec()).unwrap()),
		*split_tag
			.get_items(ItemKey::MusicBrainzRecordingId)
			.next()
			.unwrap()
			.value()
	);

	let id3v2 = split_remainder.merge_tag(split_tag);
	assert_eq!(2, id3v2.len());
	match &id3v2.frames[..] {
		[
			Frame::UniqueFileIdentifier(UniqueFileIdentifierFrame {
				owner: first_owner,
				identifier: first_identifier,
				..
			}),
			Frame::UniqueFileIdentifier(UniqueFileIdentifierFrame {
				owner: second_owner,
				identifier: second_identifier,
				..
			}),
		] => {
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

#[test_log::test]
fn get_set_user_defined_text() {
	let description = String::from("FOO_BAR");
	let content = String::from("Baz!\0Qux!");
	let description2 = String::from("FOO_BAR_2");
	let content2 = String::new();

	let mut id3v2 = Id3v2Tag::default();
	let txxx_frame = Frame::UserText(ExtendedTextFrame::new(
		TextEncoding::UTF8,
		description.clone(),
		content.clone(),
	));

	id3v2.insert(txxx_frame.clone());

	// Insert another to verify we can search through multiple
	let txxx_frame2 = Frame::UserText(ExtendedTextFrame::new(
		TextEncoding::UTF8,
		description2.clone(),
		content2.clone(),
	));
	id3v2.insert(txxx_frame2);

	// We cannot get user defined texts through `get_text`
	assert!(
		id3v2
			.get_text(&FrameId::Valid(Cow::Borrowed("TXXX")))
			.is_none()
	);

	assert_eq!(id3v2.get_user_text(description.as_str()), Some(&*content));

	// Wipe the tag
	id3v2.clear();

	// Same thing process as above, using simplified setter
	assert!(
		id3v2
			.insert_user_text(description.clone(), content.clone())
			.is_none()
	);
	assert!(
		id3v2
			.insert_user_text(description2.clone(), content2.clone())
			.is_none()
	);
	assert_eq!(id3v2.get_user_text(description.as_str()), Some(&*content));

	// Remove one frame
	assert!(id3v2.remove_user_text(&description).is_some());
	assert!(!id3v2.is_empty());

	// Now clear the remaining item
	assert!(id3v2.remove_user_text(&description2).is_some());
	assert!(id3v2.is_empty());
}

#[test_log::test]
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

#[test_log::test]
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
	let Frame::UserUrl(url) = &url_frame else {
		panic!("Expected a UserUrl")
	};
	assert_eq!(url.content, "https://www.myfanpage.com");
}

fn id3v2_tag_with_genre(value: &str) -> Id3v2Tag {
	let mut tag = Id3v2Tag::default();
	let frame = new_text_frame(GENRE_ID, String::from(value));
	tag.insert(frame);
	tag
}

#[test_log::test]
fn genre_text() {
	let tag = id3v2_tag_with_genre("Dream Pop");
	assert_eq!(tag.genre(), Some(Cow::Borrowed("Dream Pop")));
}
#[test_log::test]
fn genre_id_brackets() {
	let tag = id3v2_tag_with_genre("(21)");
	assert_eq!(tag.genre(), Some(Cow::Borrowed("Ska")));
}

#[test_log::test]
fn genre_id_numeric() {
	let tag = id3v2_tag_with_genre("21");
	assert_eq!(tag.genre(), Some(Cow::Borrowed("Ska")));
}

#[test_log::test]
fn genre_id_multiple_joined() {
	let tag = id3v2_tag_with_genre("(51)(39)");
	assert_eq!(
		tag.genre(),
		Some(Cow::Borrowed("Techno-Industrial / Noise"))
	);
}

#[test_log::test]
fn genres_id_multiple() {
	let tag = id3v2_tag_with_genre("(51)(39)");
	let mut genres = tag.genres().unwrap();
	assert_eq!(genres.next(), Some("Techno-Industrial"));
	assert_eq!(genres.next(), Some("Noise"));
	assert_eq!(genres.next(), None);
}

#[test_log::test]
fn genres_id_multiple_into_tag() {
	let id3v2 = id3v2_tag_with_genre("(51)(39)");
	let tag: Tag = id3v2.into();
	let mut genres = tag.get_strings(ItemKey::Genre);
	assert_eq!(genres.next(), Some("Techno-Industrial"));
	assert_eq!(genres.next(), Some("Noise"));
	assert_eq!(genres.next(), None);
}

#[test_log::test]
fn genres_null_separated() {
	let tag = id3v2_tag_with_genre("Samba-rock\0MPB\0Funk");
	let mut genres = tag.genres().unwrap();
	assert_eq!(genres.next(), Some("Samba-rock"));
	assert_eq!(genres.next(), Some("MPB"));
	assert_eq!(genres.next(), Some("Funk"));
	assert_eq!(genres.next(), None);
}

#[test_log::test]
fn genres_id_textual_refinement() {
	let tag = id3v2_tag_with_genre("(4)Eurodisco");
	let mut genres = tag.genres().unwrap();
	assert_eq!(genres.next(), Some("Disco"));
	assert_eq!(genres.next(), Some("Eurodisco"));
	assert_eq!(genres.next(), None);
}

#[test_log::test]
fn genres_id_bracketed_refinement() {
	let tag = id3v2_tag_with_genre("(26)(55)((I think...)");
	let mut genres = tag.genres().unwrap();
	assert_eq!(genres.next(), Some("Ambient"));
	assert_eq!(genres.next(), Some("Dream"));
	assert_eq!(genres.next(), Some("(I think...)"));
	assert_eq!(genres.next(), None);
}

#[test_log::test]
fn genres_id_remix_cover() {
	let tag = id3v2_tag_with_genre("(0)(RX)(CR)");
	let mut genres = tag.genres().unwrap();
	assert_eq!(genres.next(), Some("Blues"));
	assert_eq!(genres.next(), Some("Remix"));
	assert_eq!(genres.next(), Some("Cover"));
	assert_eq!(genres.next(), None);
}

#[test_log::test]
fn tipl_round_trip() {
	let mut tag = Id3v2Tag::default();
	let mut tipl = KeyValueFrame::new(
		FrameId::Valid(Cow::Borrowed("TIPL")),
		TextEncoding::UTF8,
		Vec::new(),
	);

	// Add all supported keys
	for (_, key) in TIPL_MAPPINGS {
		tipl.key_value_pairs
			.push(((*key).into(), "Serial-ATA".into()));
	}

	// Add one unsupported key
	tipl.key_value_pairs.push(("Foo".into(), "Bar".into()));

	tag.insert(Frame::KeyValue(tipl.clone()));

	let (split_remainder, split_tag) = tag.split_tag();
	assert_eq!(split_remainder.0.len(), 1); // "Foo" is not supported
	assert_eq!(split_tag.len(), TIPL_MAPPINGS.len()); // All supported keys are present

	for (item_key, _) in TIPL_MAPPINGS {
		assert_eq!(
			split_tag
				.get(*item_key)
				.map(TagItem::value)
				.and_then(ItemValue::text),
			Some("Serial-ATA")
		);
	}

	let mut id3v2 = split_remainder.merge_tag(split_tag);
	assert_eq!(id3v2.frames.len(), 1);
	match &mut id3v2.frames[..] {
		[Frame::KeyValue(tipl2)] => {
			// Order will not be the same, so we have to sort first
			tipl.key_value_pairs.sort();
			tipl2.key_value_pairs.sort();
			assert_eq!(tipl, *tipl2);
		},
		_ => unreachable!(),
	}
}

#[test_log::test]
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

#[test_log::test]
fn itunes_advisory_roundtrip() {
	use crate::mp4::{AdvisoryRating, Ilst};

	let mut tag = Ilst::new();
	tag.set_advisory_rating(AdvisoryRating::Explicit);

	let tag: Tag = tag.into();
	let tag: Id3v2Tag = tag.into();

	assert_eq!(tag.frames.len(), 1);

	let frame = tag.get_user_text("ITUNESADVISORY");
	assert!(frame.is_some());
	assert_eq!(frame.unwrap(), "1");

	let tag: Tag = tag.into();
	let tag: Ilst = tag.into();

	assert_eq!(tag.advisory_rating(), Some(AdvisoryRating::Explicit));
}

#[test_log::test]
fn timestamp_roundtrip() {
	let mut tag = Id3v2Tag::default();
	tag.insert(Frame::Timestamp(TimestampFrame::new(
		FrameId::Valid(Cow::Borrowed("TDRC")),
		TextEncoding::UTF8,
		Timestamp {
			year: 2024,
			month: Some(6),
			day: Some(3),
			hour: Some(14),
			minute: Some(8),
			second: Some(49),
		},
	)));

	let tag: Tag = tag.into();
	assert_eq!(tag.len(), 1);
	assert_eq!(
		tag.get_string(ItemKey::RecordingDate),
		Some("2024-06-03T14:08:49")
	);

	let tag: Id3v2Tag = tag.into();
	assert_eq!(tag.frames.len(), 1);

	let frame = tag.frames.first().unwrap();
	assert_eq!(frame.id(), &FrameId::Valid(Cow::Borrowed("TDRC")));
	match &frame {
		Frame::Timestamp(frame) => {
			assert_eq!(frame.timestamp.year, 2024);
			assert_eq!(frame.timestamp.month, Some(6));
			assert_eq!(frame.timestamp.day, Some(3));
			assert_eq!(frame.timestamp.hour, Some(14));
			assert_eq!(frame.timestamp.minute, Some(8));
			assert_eq!(frame.timestamp.second, Some(49));
		},
		_ => panic!("Expected a TimestampFrame"),
	}
}

#[test_log::test]
fn special_items_roundtrip() {
	let mut tag = Id3v2Tag::new();

	let rva2 = Frame::RelativeVolumeAdjustment(RelativeVolumeAdjustmentFrame::new(
		String::from("Foo RVA"),
		Cow::Owned(HashMap::from([(
			ChannelType::MasterVolume,
			ChannelInformation {
				channel_type: ChannelType::MasterVolume,
				volume_adjustment: 30,
				bits_representing_peak: 0,
				peak_volume: None,
			},
		)])),
	));

	tag.insert(rva2.clone());
	tag.set_artist(String::from("Foo Artist")); // Some value that we *can* represent generically

	let tag: Tag = tag.into();

	assert_eq!(tag.len(), 1);
	assert_eq!(tag.artist().as_deref(), Some("Foo Artist"));

	let mut tag: Id3v2Tag = tag.into();

	assert_eq!(tag.frames.len(), 2);
	assert_eq!(tag.artist().as_deref(), Some("Foo Artist"));
	assert_eq!(tag.get(&FrameId::Valid(Cow::Borrowed("RVA2"))), Some(&rva2));

	let mut tag_bytes = Vec::new();
	tag.dump_to(&mut tag_bytes, WriteOptions::default())
		.unwrap();

	let mut tag_re_read = read_tag_with_options(
		&tag_bytes[..],
		ParseOptions::new().parsing_mode(ParsingMode::Strict),
	);

	// Ensure ordered comparison
	tag.frames.sort_by_key(|frame| frame.id().to_string());
	tag_re_read
		.frames
		.sort_by_key(|frame| frame.id().to_string());
	assert_eq!(tag, tag_re_read);

	// Now write from `Tag`
	let tag: Tag = tag.into();

	let mut tag_bytes = Vec::new();
	tag.dump_to(&mut tag_bytes, WriteOptions::default())
		.unwrap();

	let mut generic_tag_re_read = read_tag_with_options(
		&tag_bytes[..],
		ParseOptions::new().parsing_mode(ParsingMode::Strict),
	);

	generic_tag_re_read
		.frames
		.sort_by_key(|frame| frame.id().to_string());
	assert_eq!(tag_re_read, generic_tag_re_read);
}

#[test_log::test]
fn preserve_comment_lang_description_on_conversion() {
	let mut tag = Id3v2Tag::new();

	let comment_frame = Frame::Comment(CommentFrame::new(
		TextEncoding::UTF8,
		ENGLISH,
		String::from("Some description"),
		String::from("Foo comment"),
	));

	tag.insert(comment_frame.clone());

	let tag: Tag = tag.into();
	assert_eq!(tag.len(), 1);

	let tag: Id3v2Tag = tag.into();
	assert_eq!(tag.len(), 1);

	let frame = tag.get(&FrameId::Valid(Cow::Borrowed("COMM"))).unwrap();
	match frame {
		Frame::Comment(comm) => {
			assert_eq!(comm.language, ENGLISH);
			assert_eq!(comm.description, "Some description");
			assert_eq!(comm.content, "Foo comment");
		},
		_ => panic!("Expected a CommentFrame"),
	}
}

// TODO: Remove this once we have a better solution
#[test_log::test]
fn hold_back_4_character_txxx_description() {
	let mut tag = Id3v2Tag::new();

	let _ = tag.insert_user_text(String::from("MODE"), String::from("CBR"));

	let tag: Tag = tag.into();
	assert_eq!(tag.len(), 0);

	let tag: Id3v2Tag = tag.into();
	assert_eq!(tag.len(), 1);
}

#[test_log::test]
fn skip_reading_cover_art() {
	let p = Picture::unchecked(std::iter::repeat_n(0, 50).collect::<Vec<u8>>())
		.pic_type(PictureType::CoverFront)
		.mime_type(MimeType::Jpeg)
		.build();

	let mut tag = Tag::new(TagType::Id3v2);
	tag.push_picture(p);

	tag.set_artist(String::from("Foo artist"));

	let mut writer = Vec::new();
	tag.dump_to(&mut writer, WriteOptions::new()).unwrap();

	let id3v2 = read_tag_with_options(&writer[..], ParseOptions::new().read_cover_art(false));
	assert_eq!(id3v2.len(), 1); // Artist, no picture
	assert!(id3v2.artist().is_some());
}

#[test_log::test]
fn remove_id3v24_frames_on_id3v23_save() {
	let mut tag = Id3v2Tag::new();

	tag.insert(Frame::RelativeVolumeAdjustment(
		RelativeVolumeAdjustmentFrame::new(
			String::from("Foo RVA"),
			Cow::Owned(HashMap::from([(
				ChannelType::MasterVolume,
				ChannelInformation {
					channel_type: ChannelType::MasterVolume,
					volume_adjustment: 30,
					bits_representing_peak: 0,
					peak_volume: None,
				},
			)])),
		),
	));

	let tag_re_read = dump_and_re_read(&tag, WriteOptions::default().use_id3v23(true));

	assert_eq!(tag_re_read.frames.len(), 0);
}

#[test_log::test]
fn change_text_encoding_on_id3v23_save() {
	let mut tag = Id3v2Tag::new();

	// UTF-16 BE is not supported in ID3v2.3
	tag.insert(Frame::Text(TextInformationFrame::new(
		FrameId::Valid(Cow::from("TFOO")),
		TextEncoding::UTF16BE,
		String::from("Foo"),
	)));

	let tag_re_read = dump_and_re_read(&tag, WriteOptions::default().use_id3v23(true));

	let frame = tag_re_read
		.get(&FrameId::Valid(Cow::Borrowed("TFOO")))
		.unwrap();
	match frame {
		Frame::Text(frame) => {
			assert_eq!(frame.encoding, TextEncoding::UTF16);
			assert_eq!(frame.value, "Foo");
		},
		_ => panic!("Expected a TextInformationFrame"),
	}
}

#[test_log::test]
fn split_tdor_on_id3v23_save() {
	let mut tag = Id3v2Tag::new();

	// ID3v2.3 ONLY supports the original release year.
	// This will be written as a TORY frame. Lofty just automatically upgrades it to a TDOR
	// when reading it back.
	tag.insert(Frame::Timestamp(TimestampFrame::new(
		FrameId::Valid(Cow::Borrowed("TDOR")),
		TextEncoding::UTF8,
		Timestamp {
			year: 2024,
			month: Some(6),
			day: Some(3),
			hour: Some(14),
			minute: Some(8),
			second: Some(49),
		},
	)));

	let tag_re_read = dump_and_re_read(&tag, WriteOptions::default().use_id3v23(true));

	let frame = tag_re_read
		.get(&FrameId::Valid(Cow::Borrowed("TDOR")))
		.unwrap();
	match frame {
		Frame::Timestamp(frame) => {
			assert_eq!(frame.encoding, TextEncoding::UTF16);
			assert_eq!(frame.timestamp.year, 2024);
			assert_eq!(frame.timestamp.month, None);
			assert_eq!(frame.timestamp.day, None);
			assert_eq!(frame.timestamp.hour, None);
			assert_eq!(frame.timestamp.minute, None);
			assert_eq!(frame.timestamp.second, None);
		},
		_ => panic!("Expected a TimestampFrame"),
	}
}

#[test_log::test]
fn split_tdrc_on_id3v23_save() {
	let mut tag = Id3v2Tag::new();

	// TDRC gets split into 3 frames in ID3v2.3:
	//
	// TYER: YYYY
	// TDAT: DDMM
	// TIME: HHMM
	tag.insert(Frame::Timestamp(TimestampFrame::new(
		FrameId::Valid(Cow::Borrowed("TDRC")),
		TextEncoding::UTF8,
		Timestamp {
			year: 2024,
			month: Some(6),
			day: Some(3),
			hour: Some(14),
			minute: Some(8),
			second: None, // Seconds are not supported in ID3v2.3 TIME
		},
	)));

	let tag_re_read = dump_and_re_read(&tag, WriteOptions::default().use_id3v23(true));

	// First, check the default behavior which should return the same TDRC frame
	let frame = tag_re_read
		.get(&FrameId::Valid(Cow::Borrowed("TDRC")))
		.unwrap();

	match frame {
		Frame::Timestamp(frame) => {
			assert_eq!(frame.encoding, TextEncoding::UTF16);
			assert_eq!(frame.timestamp.year, 2024);
			assert_eq!(frame.timestamp.month, Some(6));
			assert_eq!(frame.timestamp.day, Some(3));
			assert_eq!(frame.timestamp.hour, Some(14));
			assert_eq!(frame.timestamp.minute, Some(8));
		},
		_ => panic!("Expected a TimestampFrame"),
	}

	// Now, re-read with implicit_conversions off, which retains the split frames
	let mut bytes = Cursor::new(Vec::new());
	tag_re_read
		.dump_to(&mut bytes, WriteOptions::default().use_id3v23(true))
		.unwrap();

	let tag_re_read = read_tag_with_options(
		&bytes.into_inner(),
		ParseOptions::new()
			.parsing_mode(ParsingMode::Strict)
			.implicit_conversions(false),
	);

	let year = tag_re_read
		.get_text(&FrameId::Valid(Cow::Borrowed("TYER")))
		.expect("Expected TYER frame");
	assert_eq!(year, "2024");

	let date = tag_re_read
		.get_text(&FrameId::Valid(Cow::Borrowed("TDAT")))
		.expect("Expected TDAT frame");
	assert_eq!(date, "0306");

	let time = tag_re_read
		.get_text(&FrameId::Valid(Cow::Borrowed("TIME")))
		.expect("Expected TIME frame");
	assert_eq!(time, "1408");
}

#[test_log::test]
fn artists_tag_conversion() {
	const ARTISTS: &[&str] = &["Foo", "Bar", "Baz"];

	let mut tag = Tag::new(TagType::Id3v2);

	for artist in ARTISTS {
		tag.push(TagItem::new(
			ItemKey::TrackArtists,
			ItemValue::Text((*artist).to_string()),
		));
	}

	let tag: Id3v2Tag = tag.into();
	let txxx_artists = tag.get_user_text("ARTISTS").unwrap();
	let id3v2_artists = txxx_artists.split('\0').collect::<Vec<_>>();

	assert_eq!(id3v2_artists, ARTISTS);
}

#[test_log::test]
fn ensure_frame_skipping_within_bounds() {
	// This tag has an invalid `TDEN` frame, but it is skippable in BestAttempt/Relaxed parsing mode.
	// We should be able to continue reading the tag as normal, reaching the other `TDTG` frame.

	let path = "tests/tags/assets/id3v2/skippable_frame_otherwise_valid.id3v24";
	let tag = read_tag_with_options(
		&read_path(path),
		ParseOptions::new().parsing_mode(ParsingMode::BestAttempt),
	);

	assert_eq!(tag.len(), 1);
	assert_eq!(
		tag.get(&FrameId::Valid(Cow::Borrowed("TDTG"))),
		Some(&Frame::Timestamp(TimestampFrame::new(
			FrameId::Valid(Cow::Borrowed("TDTG")),
			TextEncoding::Latin1,
			Timestamp {
				year: 2014,
				month: Some(6),
				day: Some(10),
				hour: Some(2),
				minute: Some(16),
				second: Some(10),
			},
		)))
	);
}

#[test_log::test]
fn multi_item_tag_dump() {
	let mut tag = Tag::new(TagType::Id3v2);

	tag.push(TagItem::new(
		ItemKey::TrackArtist,
		ItemValue::Text(String::from("Foo")),
	));
	tag.push(TagItem::new(
		ItemKey::TrackArtist,
		ItemValue::Text(String::from("Bar")),
	));

	let mut id3v2 = Vec::new();
	tag.dump_to(&mut id3v2, WriteOptions::default()).unwrap();

	let tag = read_tag_with_options(
		&id3v2,
		ParseOptions::new().parsing_mode(ParsingMode::Strict),
	);

	// Both artists should be merged into a single frame
	assert_eq!(tag.len(), 1);

	let artist_tag = tag.get_text(&FrameId::new("TPE1").unwrap()).unwrap();
	assert_eq!(artist_tag, "Foo\0Bar");
}

#[test_log::test]
fn single_value_frame() {
	let mut tag = Tag::new(TagType::Id3v2);

	// TBPM should be deduplicated during the conversion, taking whatever happens to be first
	tag.push(TagItem::new(
		ItemKey::IntegerBpm,
		ItemValue::Text(String::from("120")),
	));
	tag.push(TagItem::new(
		ItemKey::IntegerBpm,
		ItemValue::Text(String::from("130")),
	));
	tag.push(TagItem::new(
		ItemKey::IntegerBpm,
		ItemValue::Text(String::from("140")),
	));

	let mut id3v2 = Vec::new();
	tag.dump_to(&mut id3v2, WriteOptions::default()).unwrap();

	let tag = read_tag_with_options(
		&id3v2,
		ParseOptions::new().parsing_mode(ParsingMode::Strict),
	);

	// The other BPM values were discarded, **NOT** merged
	assert_eq!(tag.len(), 1);

	let artist_tag = tag.get_text(&FrameId::new("TBPM").unwrap()).unwrap();
	assert_eq!(artist_tag, "120");
}

macro_rules! popm_tests {
		(
			$($tagger_name:ident => $(($tagger_value:expr, $mapped_value:literal)),+);* $(;)?
		) => {
			paste::paste! {
				$(
				#[test]
				fn [<popm_ $tagger_name>]() {
					$(
					let popularimeter = Popularimeter::$tagger_name($tagger_value, 0);
					let frame = PopularimeterFrame::from(popularimeter);
					assert_eq!(frame.rating, $mapped_value, "Expected {} to map to {}", stringify!($tagger_value), $mapped_value);
					)+
				}
				)*
			}
		}
	}

popm_tests! {
	musicbee =>
	(StarRating::One, 1),
	(StarRating::Two, 64),
	(StarRating::Three, 128),
	(StarRating::Four, 196),
	(StarRating::Five, 255);
	windows_media_player =>
	(StarRating::One, 1),
	(StarRating::Two, 64),
	(StarRating::Three, 128),
	(StarRating::Four, 196),
	(StarRating::Five, 255);
	picard =>
	(StarRating::One, 51),
	(StarRating::Two, 102),
	(StarRating::Three, 153),
	(StarRating::Four, 204),
	(StarRating::Five, 255);
}
