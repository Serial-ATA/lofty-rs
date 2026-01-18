use crate::temp_file;

use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{Read, Seek};

use lofty::TextEncoding;
use lofty::config::{ParseOptions, ParsingMode, WriteOptions};
use lofty::file::AudioFile;
use lofty::id3::v2::{
	AttachedPictureFrame, ChannelInformation, ChannelType, CommentFrame, Event,
	EventTimingCodesFrame, EventType, ExtendedTextFrame, ExtendedUrlFrame, Frame, FrameFlags,
	FrameId, GeneralEncapsulatedObject, Id3v2Tag, Id3v2Version, KeyValueFrame, OwnershipFrame,
	PopularimeterFrame, PrivateFrame, RelativeVolumeAdjustmentFrame, SyncTextContentType,
	SynchronizedTextFrame, TextInformationFrame, TimestampFormat, TimestampFrame,
	UniqueFileIdentifierFrame, UnsynchronizedTextFrame, UrlLinkFrame,
};
use lofty::mpeg::MpegFile;
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::tag::items::Timestamp;
use lofty::tag::{Accessor, TagExt};

#[test_log::test]
fn test_unsynch_decode() {
	let mut file = temp_file!("tests/taglib/data/unsynch.id3");
	let f = MpegFile::read_from(&mut file, ParseOptions::new().read_properties(false)).unwrap();

	assert!(f.id3v2().is_some());
	assert_eq!(
		f.id3v2().unwrap().title().as_deref(),
		Some("My babe just cares for me")
	);
}

#[test_log::test]
fn test_downgrade_utf8_for_id3v23_1() {
	let mut file = temp_file!("tests/taglib/data/xing.mp3");

	let f = TextInformationFrame::new(
		FrameId::Valid(Cow::Borrowed("TPE1")),
		TextEncoding::UTF8,
		String::from("Foo"),
	);

	let mut id3v2 = Id3v2Tag::new();
	id3v2.insert(Frame::Text(f.clone()));
	id3v2
		.save_to(&mut file, WriteOptions::new().use_id3v23(true))
		.unwrap();

	let data = f
		.as_bytes(WriteOptions::default().use_id3v23(true))
		.unwrap();
	assert_eq!(data.len(), 1 + 6 + 2); // NOTE: This does not include frame headers like TagLib does

	let f2 = TextInformationFrame::parse(
		&mut &data[..],
		FrameId::Valid(Cow::Borrowed("TPE1")),
		FrameFlags::default(),
		Id3v2Version::V3,
	)
	.unwrap()
	.unwrap();

	assert_eq!(f.value, f2.value);
	assert_eq!(f2.encoding, TextEncoding::UTF16);
}

#[test_log::test]
fn test_downgrade_utf8_for_id3v23_2() {
	let mut file = temp_file!("tests/taglib/data/xing.mp3");

	let f = UnsynchronizedTextFrame::new(
		TextEncoding::UTF8,
		*b"XXX",
		String::new(),
		String::from("Foo"),
	);

	let mut id3v2 = Id3v2Tag::new();
	id3v2.insert(Frame::UnsynchronizedText(f.clone()));
	id3v2
		.save_to(&mut file, WriteOptions::new().use_id3v23(true))
		.unwrap();

	let data = f
		.as_bytes(WriteOptions::default().use_id3v23(true))
		.unwrap();
	assert_eq!(data.len(), 1 + 3 + 2 + 2 + 6 + 2); // NOTE: This does not include frame headers like TagLib does

	let f2 =
		UnsynchronizedTextFrame::parse(&mut &data[..], FrameFlags::default(), Id3v2Version::V3)
			.unwrap()
			.unwrap();

	assert_eq!(f2.content, String::from("Foo"));
	assert_eq!(f2.encoding, TextEncoding::UTF16);
}

#[test_log::test]
fn test_utf16be_delimiter() {
	let mut f = TextInformationFrame::new(
		FrameId::Valid(Cow::Borrowed("TIT2")),
		TextEncoding::UTF16BE,
		String::from("Foo\0Bar"),
	);

	let data = f.as_bytes(WriteOptions::default()).unwrap();

	let no_bom_be_data = b"\x02\
	\0F\0o\0o\0\0\
	\0B\0a\0r";

	assert_eq!(data, no_bom_be_data);
	f = TextInformationFrame::parse(
		&mut &data[..],
		FrameId::Valid(Cow::Borrowed("TIT2")),
		FrameFlags::default(),
		Id3v2Version::V4,
	)
	.unwrap()
	.unwrap();
	assert_eq!(f.value, "Foo\0Bar");
}

#[test_log::test]
fn test_utf16_delimiter() {
	let mut f = TextInformationFrame::new(
		FrameId::Valid(Cow::Borrowed("TIT2")),
		TextEncoding::UTF16,
		String::from("Foo\0Bar"),
	);

	let data = f.as_bytes(WriteOptions::default()).unwrap();

	// TODO: TagLib writes a BOM to every string, making the output identical to `mutli_bom_le_data`,
	//       rather than `single_bom_le_data` in Lofty's case. Not sure if we should be writing the BOM
	//       to every string?
	let single_bom_le_data = b"\x01\xff\xfe\
                              F\0o\0o\0\0\0\
                              B\0a\0r\0";

	assert_eq!(data, single_bom_le_data);
	f = TextInformationFrame::parse(
		&mut &data[..],
		FrameId::Valid(Cow::Borrowed("TIT2")),
		FrameFlags::default(),
		Id3v2Version::V4,
	)
	.unwrap()
	.unwrap();
	assert_eq!(f.value, "Foo\0Bar");

	let multi_bom_le_data = b"\x01\xff\xfe\
                              F\0o\0o\0\0\0\xff\xfe\
                              B\0a\0r\0";
	f = TextInformationFrame::parse(
		&mut &multi_bom_le_data[..],
		FrameId::Valid(Cow::Borrowed("TIT2")),
		FrameFlags::default(),
		Id3v2Version::V4,
	)
	.unwrap()
	.unwrap();
	assert_eq!(f.value, "Foo\0Bar");

	let multi_bom_be_data = b"\x01\xfe\xff\
							  \0F\0o\0o\0\0\xfe\xff\
                              \0B\0a\0r";
	f = TextInformationFrame::parse(
		&mut &multi_bom_be_data[..],
		FrameId::Valid(Cow::Borrowed("TIT2")),
		FrameFlags::default(),
		Id3v2Version::V4,
	)
	.unwrap()
	.unwrap();
	assert_eq!(f.value, "Foo\0Bar");

	let single_bom_be_data = b"\x01\xfe\xff\
							  \0F\0o\0o\0\0\
							  \0B\0a\0r";
	f = TextInformationFrame::parse(
		&mut &single_bom_be_data[..],
		FrameId::Valid(Cow::Borrowed("TIT2")),
		FrameFlags::default(),
		Id3v2Version::V4,
	)
	.unwrap()
	.unwrap();
	assert_eq!(f.value, "Foo\0Bar");
}

#[test_log::test]
#[ignore = "iTunes bug that isn't handled yet"]
fn test_broken_frame1() {
	// TODO: Determine if it is worth supporting unsychronized frame sizes in ID3v2.4
	//       This is apparently an issue iTunes had at some point in the past.
	// let mut file = temp_file!("tests/taglib/data/broken-tenc.id3");
	// let f = MpegFile::read_from(&mut file, ParseOptions::new().read_properties(false)).unwrap();
	//
	// assert!(f
	// 	.id3v2()
	// 	.unwrap()
	// 	.contains(&FrameId::Valid(Cow::from("TENC"))));
}

#[test_log::test]
#[ignore = "Marker test, this is not an API Lofty replicates"]
fn test_read_string_field() {}

#[test_log::test]
fn test_parse_apic() {
	let f = AttachedPictureFrame::parse(
		&mut &b"\
	\x00\
	m\x00\
	\x01\
	d\x00\
	\x00"[..],
		FrameFlags::default(),
		Id3v2Version::V4,
	)
	.unwrap();
	assert_eq!(
		f.picture.mime_type(),
		Some(&MimeType::Unknown(String::from("m")))
	);
	assert_eq!(f.picture.pic_type(), PictureType::Icon);
	assert_eq!(f.picture.description(), Some("d"));
}

#[test_log::test]
fn test_parse_apic_utf16_bom() {
	let f = AttachedPictureFrame::parse(
		&mut &b"\
	\x01\x69\x6d\x61\x67\x65\
	\x2f\x6a\x70\x65\x67\x00\x00\xfe\xff\x00\x63\x00\x6f\x00\x76\x00\
	\x65\x00\x72\x00\x2e\x00\x6a\x00\x70\x00\x67\x00\x00\xff\xd8\xff"[..],
		FrameFlags::default(),
		Id3v2Version::V4,
	)
	.unwrap();

	assert_eq!(f.picture.mime_type(), Some(&MimeType::Jpeg));
	assert_eq!(f.picture.pic_type(), PictureType::Other);
	assert_eq!(f.picture.description(), Some("cover.jpg"));
	assert_eq!(f.picture.data(), b"\xff\xd8\xff");
}

#[test_log::test]
fn test_parse_apicv22() {
	let frame = AttachedPictureFrame::parse(
		&mut &b"\
	\x00\
	JPG\
	\x01\
	d\x00\
	\x00"[..],
		FrameFlags::default(),
		Id3v2Version::V2,
	)
	.unwrap();

	assert_eq!(frame.picture.mime_type(), Some(&MimeType::Jpeg));
	assert_eq!(frame.picture.pic_type(), PictureType::Icon);
	assert_eq!(frame.picture.description(), Some("d"));
}

#[test_log::test]
fn test_render_apic() {
	let f = AttachedPictureFrame::new(
		TextEncoding::UTF8,
		Picture::unchecked(b"PNG data".to_vec())
			.pic_type(PictureType::CoverBack)
			.mime_type(MimeType::Png)
			.description("Description")
			.build(),
	);

	assert_eq!(
		f.as_bytes(WriteOptions::default()).unwrap(),
		b"\
	\x03\
	image/png\x00\
	\x04\
	Description\x00\
	PNG data"
	);
}

#[test_log::test]
#[ignore = "Marker test, not sure what's going on here?"]
fn test_dont_render22() {}

#[test_log::test]
fn test_parse_geob() {
	let f = GeneralEncapsulatedObject::parse(
		b"\
	\x00\
	m\x00\
	f\x00\
	d\x00\
	\x00",
		FrameFlags::default(),
	)
	.unwrap();
	assert_eq!(f.mime_type.as_deref(), Some("m"));
	assert_eq!(f.file_name.as_deref(), Some("f"));
	assert_eq!(f.descriptor.as_deref(), Some("d"));
}

#[test_log::test]
fn test_render_geob() {
	let f = GeneralEncapsulatedObject::new(
		TextEncoding::Latin1,
		Some(String::from("application/octet-stream")),
		Some(String::from("test.bin")),
		Some(String::from("Description")),
		vec![0x01; 3],
	);

	assert_eq!(
		f.as_bytes(WriteOptions::default()).unwrap(),
		b"\
	\x00\
	application/octet-stream\x00\
	test.bin\x00\
	Description\x00\
	\x01\x01\x01"
	);
}

#[test_log::test]
fn test_parse_popm() {
	let f = PopularimeterFrame::parse(
		&mut &b"\
	email@example.com\x00\
	\x02\
	\x00\x00\x00\x03"[..],
		FrameFlags::default(),
	)
	.unwrap();
	assert_eq!(f.email, "email@example.com");
	assert_eq!(f.rating, 2);
	assert_eq!(f.counter, 3);
}

#[test_log::test]
fn test_parse_popm_without_counter() {
	let f = PopularimeterFrame::parse(
		&mut &b"\
	email@example.com\x00\
	\x02"[..],
		FrameFlags::default(),
	)
	.unwrap();
	assert_eq!(f.email, "email@example.com");
	assert_eq!(f.rating, 2);
	assert_eq!(f.counter, 0);
}

#[test_log::test]
fn test_render_popm() {
	let f = PopularimeterFrame::new(String::from("email@example.com"), 2, 3);

	assert_eq!(
		f.as_bytes(WriteOptions::default()).unwrap(),
		b"\
	email@example.com\x00\
	\x02\
	\x00\x00\x00\x03"
	);
}

#[test_log::test]
#[ignore = "Marker test, Lofty doesn't have a display impl for Popularimeter"]
fn test_popm_to_string() {}

#[test_log::test]
fn test_popm_from_file() {
	let mut file = temp_file!("tests/taglib/data/xing.mp3");

	let f = PopularimeterFrame::new(String::from("email@example.com"), 200, 3);

	{
		let mut foo = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		let mut tag = Id3v2Tag::new();
		tag.insert(Frame::Popularimeter(f));
		foo.set_id3v2(tag);
		foo.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let bar = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		let popm_frame = bar
			.id3v2()
			.unwrap()
			.get(&FrameId::Valid(Cow::Borrowed("POPM")))
			.unwrap();
		let Frame::Popularimeter(popularimeter) = popm_frame else {
			unreachable!()
		};

		assert_eq!(popularimeter.email, "email@example.com");
		assert_eq!(popularimeter.rating, 200);
	}
}

#[test_log::test]
#[allow(clippy::float_cmp)]
fn test_parse_relative_volume_frame() {
	let f = RelativeVolumeAdjustmentFrame::parse(
		&mut &b"\
	ident\x00\
    \x02\
    \x00\x0F\
    \x08\
    \x45"[..],
		FrameFlags::default(),
		ParsingMode::Strict,
	)
	.unwrap()
	.unwrap();

	assert_eq!(f.identification, "ident");
	let front_right = f.channels.get(&ChannelType::FrontRight).unwrap();
	assert_eq!(
		f32::from(front_right.volume_adjustment) / 512.0f32,
		15.0f32 / 512.0f32
	);
	assert_eq!(front_right.volume_adjustment, 15);
	assert_eq!(front_right.bits_representing_peak, 8);
	assert_eq!(front_right.peak_volume, Some(vec![0x45]));
	let channels = f.channels;
	assert_eq!(channels.len(), 1);
}

#[test_log::test]
fn test_render_relative_volume_frame() {
	let f = RelativeVolumeAdjustmentFrame::new("ident", {
		let mut m = HashMap::new();
		m.insert(
			ChannelType::FrontRight,
			ChannelInformation {
				channel_type: ChannelType::FrontRight,
				volume_adjustment: 15,
				bits_representing_peak: 8,
				peak_volume: Some(vec![0x45]),
			},
		);
		Cow::Owned(m)
	});

	assert_eq!(
		f.as_bytes(WriteOptions::default()).unwrap(),
		b"\
	ident\x00\
    \x02\
    \x00\x0F\
    \x08\
    \x45"
	);
}

#[test_log::test]
fn test_parse_unique_file_identifier_frame() {
	let f = UniqueFileIdentifierFrame::parse(
		&mut &b"\
	owner\x00\
	\x00\x01\x02"[..],
		FrameFlags::default(),
		ParsingMode::Strict,
	)
	.unwrap()
	.unwrap();

	assert_eq!(f.owner, "owner");
	assert_eq!(&*f.identifier, &[0x00, 0x01, 0x02]);
}

#[test_log::test]
fn test_parse_empty_unique_file_identifier_frame() {
	let f = UniqueFileIdentifierFrame::parse(
		&mut &b"\
	\x00\
	"[..],
		FrameFlags::default(),
		ParsingMode::Strict,
	);

	// NOTE: TagLib considers a missing owner to be valid, we do not
	assert!(f.is_err());
}

#[test_log::test]
fn test_render_unique_file_identifier_frame() {
	let f = UniqueFileIdentifierFrame::new(String::from("owner"), b"\x01\x02\x03".to_vec());

	assert_eq!(
		f.as_bytes(WriteOptions::default()).unwrap(),
		b"\
owner\x00\
\x01\x02\x03"
	);
}

#[test_log::test]
fn test_parse_url_link_frame() {
	let f = UrlLinkFrame::parse(
		&mut &b"http://example.com"[..],
		FrameId::Valid(Cow::Borrowed("WPUB")),
		FrameFlags::default(),
	)
	.unwrap()
	.unwrap();
	assert_eq!(f.url(), "http://example.com");
}

#[test_log::test]
fn test_render_url_link_frame() {
	let f = UrlLinkFrame::parse(
		&mut &b"http://example.com"[..],
		FrameId::Valid(Cow::Borrowed("WPUB")),
		FrameFlags::default(),
	)
	.unwrap()
	.unwrap();
	assert_eq!(
		f.as_bytes(WriteOptions::default()).unwrap(),
		b"http://example.com"
	);
}

#[test_log::test]
fn test_parse_user_url_link_frame() {
	let f = ExtendedUrlFrame::parse(
		&mut &b"\
	\x00\
	foo\x00\
	http://example.com"[..],
		FrameFlags::default(),
		Id3v2Version::V4,
	)
	.unwrap()
	.unwrap();

	assert_eq!(f.description, String::from("foo"));
	assert_eq!(f.content, String::from("http://example.com"));
}

#[test_log::test]
fn test_render_user_url_link_frame() {
	let f = ExtendedUrlFrame::new(
		TextEncoding::Latin1,
		String::from("foo"),
		String::from("http://example.com"),
	);

	assert_eq!(
		f.as_bytes(WriteOptions::default()).unwrap(),
		b"\
	\x00\
	foo\x00\
	http://example.com"
	);
}

#[test_log::test]
fn test_parse_ownership_frame() {
	let f = OwnershipFrame::parse(
		&mut &b"\
		\x00\
        GBP1.99\x00\
		20120905\
		Beatport"[..],
		FrameFlags::default(),
	)
	.unwrap()
	.unwrap();

	assert_eq!(f.price_paid, "GBP1.99");
	assert_eq!(f.date_of_purchase, "20120905");
	assert_eq!(f.seller, "Beatport");
}

#[test_log::test]
fn test_render_ownership_frame() {
	let f = OwnershipFrame::new(
		TextEncoding::Latin1,
		String::from("GBP1.99"),
		String::from("20120905"),
		String::from("Beatport"),
	);

	assert_eq!(
		f.as_bytes(WriteOptions::default()).unwrap(),
		b"\
		\x00\
        GBP1.99\x00\
		20120905\
		Beatport"[..]
	)
}

#[test_log::test]
fn test_parse_synchronized_lyrics_frame() {
	let f = SynchronizedTextFrame::parse(
		b"\
	\x00\
eng\
\x02\
\x01\
foo\x00\
Example\x00\
\x00\x00\x04\xd2\
Lyrics\x00\
\x00\x00\x11\xd7",
		FrameFlags::default(),
	)
	.unwrap();

	assert_eq!(f.encoding, TextEncoding::Latin1);
	assert_eq!(f.language, *b"eng");
	assert_eq!(f.timestamp_format, TimestampFormat::MS);
	assert_eq!(f.content_type, SyncTextContentType::Lyrics);
	assert_eq!(f.description.as_deref(), Some("foo"));

	assert_eq!(f.content.len(), 2);
	assert_eq!(f.content[0].1, "Example");
	assert_eq!(f.content[0].0, 1234);
	assert_eq!(f.content[1].1, "Lyrics");
	assert_eq!(f.content[1].0, 4567);
}

#[test_log::test]
fn test_parse_synchronized_lyrics_frame_with_empty_description() {
	let f = SynchronizedTextFrame::parse(
		b"\
	\x00\
	eng\
	\x02\
	\x01\
	\x00\
	Example\x00\
	\x00\x00\x04\xd2\
	Lyrics\x00\
	\x00\x00\x11\xd7",
		FrameFlags::default(),
	)
	.unwrap();

	assert_eq!(f.encoding, TextEncoding::Latin1);
	assert_eq!(f.language, *b"eng");
	assert_eq!(f.timestamp_format, TimestampFormat::MS);
	assert_eq!(f.content_type, SyncTextContentType::Lyrics);
	assert!(f.description.is_none());

	assert_eq!(f.content.len(), 2);
	assert_eq!(f.content[0].1, "Example");
	assert_eq!(f.content[0].0, 1234);
	assert_eq!(f.content[1].1, "Lyrics");
	assert_eq!(f.content[1].0, 4567);
}

#[test_log::test]
fn test_render_synchronized_lyrics_frame() {
	let f = SynchronizedTextFrame::new(
		TextEncoding::Latin1,
		*b"eng",
		TimestampFormat::MS,
		SyncTextContentType::Lyrics,
		Some(String::from("foo")),
		vec![
			(1234, String::from("Example")),
			(4567, String::from("Lyrics")),
		],
	);

	assert_eq!(
		f.as_bytes(WriteOptions::default()).unwrap(),
		b"\
	\x00\
	eng\
	\x02\
	\x01\
	foo\x00\
	Example\x00\
	\x00\x00\x04\xd2\
	Lyrics\x00\
	\x00\x00\x11\xd7"
	);
}

#[test_log::test]
fn test_parse_event_timing_codes_frame() {
	let f = EventTimingCodesFrame::parse(
		&mut &b"\
	\x02\
	\x02\
	\x00\x00\xf3\x5c\
	\xfe\
	\x00\x36\xee\x80"[..],
		FrameFlags::default(),
	)
	.unwrap()
	.unwrap();

	assert_eq!(f.timestamp_format, TimestampFormat::MS);

	let sel = f.events;
	assert_eq!(sel.len(), 2);
	assert_eq!(sel[0].event_type, EventType::IntroStart);
	assert_eq!(sel[0].timestamp, 62300);
	assert_eq!(sel[1].event_type, EventType::AudioFileEnds);
	assert_eq!(sel[1].timestamp, 3_600_000);
}

#[test_log::test]
fn test_render_event_timing_codes_frame() {
	let f = EventTimingCodesFrame::new(
		TimestampFormat::MS,
		vec![
			Event {
				event_type: EventType::IntroStart,
				timestamp: 62300,
			},
			Event {
				event_type: EventType::AudioFileEnds,
				timestamp: 3_600_000,
			},
		],
	);

	assert_eq!(
		f.as_bytes(),
		b"\
	\x02\
	\x02\
	\x00\x00\xf3\x5c\
	\xfe\
	\x00\x36\xee\x80"
	)
}

#[test_log::test]
fn test_parse_comments_frame() {
	let f = CommentFrame::parse(
		&mut &b"\x03\
								deu\
								Description\x00\
								Text"[..],
		FrameFlags::default(),
		Id3v2Version::V4,
	)
	.unwrap()
	.unwrap();

	assert_eq!(f.encoding, TextEncoding::UTF8);
	assert_eq!(f.language, *b"deu");
	assert_eq!(f.description, String::from("Description"));
	assert_eq!(f.content, String::from("Text"));
}

#[test_log::test]
fn test_render_comments_frame() {
	let f = CommentFrame::new(
		TextEncoding::UTF16,
		*b"eng",
		String::from("Description"),
		String::from("Text"),
	);

	assert_eq!(
		f.as_bytes(WriteOptions::default()).unwrap(),
		b"\
	\x01\
	eng\
	\xff\xfeD\0e\0s\0c\0r\0i\0p\0t\0i\0o\0n\0\x00\x00\
	\xff\xfeT\0e\0x\0t\0"
	);
}

#[test_log::test]
#[ignore = "Marker test, Lofty doesn't have dedicated support for PCST frames, it seems unnecessary"]
fn test_parse_podcast_frame() {}

#[test_log::test]
#[ignore = "Marker test, Lofty doesn't have dedicated support for PCST frames, it seems unnecessary"]
fn test_render_podcast_frame() {}

#[test_log::test]
fn test_parse_private_frame() {
	let f = PrivateFrame::parse(
		&mut &b"\
	WM/Provider\x00\
	TL"[..],
		FrameFlags::default(),
	)
	.unwrap()
	.unwrap();

	assert_eq!(f.owner, "WM/Provider");
	assert_eq!(&*f.private_data, b"TL");
}

#[test_log::test]
fn test_render_private_frame() {
	let f = PrivateFrame::new(String::from("WM/Provider"), b"TL".to_vec());

	assert_eq!(
		f.as_bytes(WriteOptions::default()).unwrap(),
		b"\
	WM/Provider\x00\
	TL"
	);
}

#[test_log::test]
fn test_parse_user_text_identification_frame() {
	let frame_without_description = ExtendedUrlFrame::parse(
		&mut &b"\
	\x00\
	\x00\
	Text"[..],
		FrameFlags::default(),
		Id3v2Version::V4,
	)
	.unwrap()
	.unwrap();

	assert_eq!(frame_without_description.description, String::new());
	assert_eq!(frame_without_description.content, String::from("Text"));

	let frame_with_description = ExtendedUrlFrame::parse(
		&mut &b"\
	\x00\
	Description\x00\
	Text"[..],
		FrameFlags::default(),
		Id3v2Version::V4,
	)
	.unwrap()
	.unwrap();
	assert_eq!(
		frame_with_description.description,
		String::from("Description")
	);
	assert_eq!(frame_with_description.content, String::from("Text"));
}

#[test_log::test]
fn test_render_user_text_identification_frame() {
	let mut f = ExtendedTextFrame::new(TextEncoding::Latin1, String::new(), String::from("Text"));

	assert_eq!(
		f.as_bytes(WriteOptions::default()).unwrap(),
		b"\
	\x00\
	\x00\
	Text"
	);

	f.description = Cow::Borrowed("Description");

	assert_eq!(
		f.as_bytes(WriteOptions::default()).unwrap(),
		b"\
	\x00\
	Description\x00\
	Text"
	);
}

// TODO: iTunes, being the great application it is writes unsynchronized integers for sizes. There's no *great* way to detect this.
#[test_log::test]
#[ignore = "iTunes bug that isn't handled yet"]
fn test_itunes_24_frame_size() {
	let mut file = temp_file!("tests/taglib/data/005411.id3");
	let f = MpegFile::read_from(&mut file, ParseOptions::new().read_properties(false)).unwrap();

	assert!(
		f.id3v2()
			.unwrap()
			.contains(&FrameId::Valid(Cow::from("TIT2")))
	);
	assert_eq!(
		f.id3v2()
			.unwrap()
			.get_text(&FrameId::Valid(Cow::Borrowed("TIT2")))
			.unwrap(),
		"Sunshine Superman"
	);
}

#[test_log::test]
fn test_save_utf16_comment() {
	let mut file = temp_file!("tests/taglib/data/xing.mp3");

	// NOTE: You can change the default encoding in TagLib, Lofty does not support this
	{
		let mut foo = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		let mut tag = Id3v2Tag::new();
		tag.insert(Frame::Comment(CommentFrame::new(
			TextEncoding::UTF16,
			*b"eng",
			String::new(),
			String::from("Test comment!"),
		)));
		foo.set_id3v2(tag);
		foo.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let bar = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		assert_eq!(
			bar.id3v2().unwrap().comment().as_deref(),
			Some("Test comment!")
		);
	}
}

// TODO: Probably won't ever support this, it's a weird edge case with
//       duplicate genres. That can be up to the caller to figure out.
#[test_log::test]
#[ignore = "Weird edge case, probably won't ever support this"]
fn test_update_genre_23_1() {
	// "Refinement" is the same as the ID3v1 genre - duplicate
	let frame_value = TextInformationFrame::parse(
		&mut &b"\x00\
	(22)Death Metal"[..],
		FrameId::Valid(Cow::Borrowed("TCON")),
		FrameFlags::default(),
		Id3v2Version::V4,
	)
	.unwrap()
	.unwrap();

	let mut tag = Id3v2Tag::new();
	tag.insert(Frame::Text(frame_value));

	let mut genres = tag.genres().unwrap();
	assert_eq!(genres.next(), Some("Death Metal"));
	assert!(genres.next().is_none());

	assert_eq!(tag.genre().as_deref(), Some("Death Metal"));
}

#[test_log::test]
fn test_update_genre23_2() {
	// "Refinement" is different from the ID3v1 genre
	let frame_value = TextInformationFrame::parse(
		&mut &b"\x00\
	(4)Eurodisco"[..],
		FrameId::Valid(Cow::Borrowed("TCON")),
		FrameFlags::default(),
		Id3v2Version::V4,
	)
	.unwrap()
	.unwrap();

	let mut tag = Id3v2Tag::new();
	tag.insert(Frame::Text(frame_value));

	let mut genres = tag.genres().unwrap();
	assert_eq!(genres.next(), Some("Disco"));
	assert_eq!(genres.next(), Some("Eurodisco"));
	assert!(genres.next().is_none());

	assert_eq!(tag.genre().as_deref(), Some("Disco / Eurodisco"));
}

#[test_log::test]
fn test_update_genre23_3() {
	// Multiple references and a refinement
	let frame_value = TextInformationFrame::parse(
		&mut &b"\x00\
	(9)(138)Viking Metal"[..],
		FrameId::Valid(Cow::Borrowed("TCON")),
		FrameFlags::default(),
		Id3v2Version::V4,
	)
	.unwrap()
	.unwrap();

	let mut tag = Id3v2Tag::new();
	tag.insert(Frame::Text(frame_value));

	let mut genres = tag.genres().unwrap();
	assert_eq!(genres.next(), Some("Metal"));
	assert_eq!(genres.next(), Some("Black Metal"));
	assert_eq!(genres.next(), Some("Viking Metal"));
	assert!(genres.next().is_none());

	assert_eq!(
		tag.genre().as_deref(),
		Some("Metal / Black Metal / Viking Metal")
	);
}

#[test_log::test]
fn test_update_genre_24() {
	let frame_value = TextInformationFrame::parse(
		&mut &b"\x00\
	14\0Eurodisco"[..],
		FrameId::Valid(Cow::Borrowed("TCON")),
		FrameFlags::default(),
		Id3v2Version::V4,
	)
	.unwrap()
	.unwrap();

	let mut tag = Id3v2Tag::new();
	tag.insert(Frame::Text(frame_value));

	assert_eq!(tag.genre().as_deref(), Some("R&B / Eurodisco"));
}

#[test_log::test]
fn test_update_date22() {
	let mut file = temp_file!("tests/taglib/data/id3v22-tda.mp3");
	let f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
	assert!(f.id3v2().is_some());
	assert_eq!(f.id3v2().unwrap().date().map(|date| date.year), Some(2010));
}

// TODO: Determine if this is even worth doing. It is just combining TYE+TDA when upgrading ID3v2.2 to 2.4
#[test_log::test]
#[ignore = "Lofty doesn't upgrade dates in ID3v2.2, for now"]
fn test_update_full_date22() {
	let mut file = temp_file!("tests/taglib/data/id3v22-tda.mp3");
	let f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
	assert!(f.id3v2().is_some());
	assert_eq!(
		f.id3v2()
			.unwrap()
			.get_text(&FrameId::Valid(Cow::Borrowed("TDRC")))
			.unwrap(),
		"2010-04-03"
	);
}

#[test_log::test]
fn test_downgrade_to_23() {
	let mut file = temp_file!("tests/taglib/data/xing.mp3");

	{
		let mut id3v2 = Id3v2Tag::new();

		id3v2.insert(Frame::Timestamp(TimestampFrame::new(
			FrameId::Valid(Cow::Borrowed("TDOR")),
			TextEncoding::Latin1,
			Timestamp::parse(&mut &b"2011-03-16"[..], ParsingMode::Strict)
				.unwrap()
				.unwrap(),
		)));

		id3v2.insert(Frame::Timestamp(TimestampFrame::new(
			FrameId::Valid(Cow::Borrowed("TDRC")),
			TextEncoding::Latin1,
			Timestamp::parse(&mut &b"2012-04-17T12:01"[..], ParsingMode::Strict)
				.unwrap()
				.unwrap(),
		)));

		id3v2.insert(Frame::KeyValue(KeyValueFrame::new(
			FrameId::Valid(Cow::Borrowed("TMCL")),
			TextEncoding::Latin1,
			vec![
				(Cow::Borrowed("Guitar"), Cow::Borrowed("Artist 1")),
				(Cow::Borrowed("Drums"), Cow::Borrowed("Artist 2")),
			],
		)));

		id3v2.insert(Frame::KeyValue(KeyValueFrame::new(
			FrameId::Valid(Cow::Borrowed("TIPL")),
			TextEncoding::Latin1,
			vec![
				(Cow::Borrowed("Producer"), Cow::Borrowed("Artist 3")),
				(Cow::Borrowed("Mastering"), Cow::Borrowed("Artist 4")),
			],
		)));

		id3v2.insert(Frame::Text(TextInformationFrame::new(
			FrameId::Valid(Cow::Borrowed("TCON")),
			TextEncoding::Latin1,
			String::from("51\x0039\x00Power Noise"),
		)));

		id3v2.insert(Frame::Text(TextInformationFrame::new(
			FrameId::Valid(Cow::Borrowed("TDRL")),
			TextEncoding::Latin1,
			String::new(),
		)));

		id3v2.insert(Frame::Text(TextInformationFrame::new(
			FrameId::Valid(Cow::Borrowed("TDTG")),
			TextEncoding::Latin1,
			String::new(),
		)));

		id3v2.insert(Frame::Text(TextInformationFrame::new(
			FrameId::Valid(Cow::Borrowed("TMOO")),
			TextEncoding::Latin1,
			String::new(),
		)));

		id3v2.insert(Frame::Text(TextInformationFrame::new(
			FrameId::Valid(Cow::Borrowed("TPRO")),
			TextEncoding::Latin1,
			String::new(),
		)));

		id3v2.insert(Frame::Text(TextInformationFrame::new(
			FrameId::Valid(Cow::Borrowed("TSOA")),
			TextEncoding::Latin1,
			String::new(),
		)));

		id3v2.insert(Frame::Text(TextInformationFrame::new(
			FrameId::Valid(Cow::Borrowed("TSOT")),
			TextEncoding::Latin1,
			String::new(),
		)));

		id3v2.insert(Frame::Text(TextInformationFrame::new(
			FrameId::Valid(Cow::Borrowed("TSST")),
			TextEncoding::Latin1,
			String::new(),
		)));

		id3v2.insert(Frame::Text(TextInformationFrame::new(
			FrameId::Valid(Cow::Borrowed("TSOP")),
			TextEncoding::Latin1,
			String::new(),
		)));

		id3v2
			.save_to(&mut file, WriteOptions::new().use_id3v23(true))
			.unwrap();
	}
	file.rewind().unwrap();
	{
		let f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		assert!(f.id3v2().is_some());

		let id3v2 = f.id3v2().unwrap();
		let tf = id3v2.get(&FrameId::Valid(Cow::Borrowed("TDOR"))).unwrap();
		let Frame::Timestamp(TimestampFrame { timestamp, .. }) = tf else {
			unreachable!()
		};
		assert_eq!(timestamp.to_string(), "2011");

		let tf = id3v2.get(&FrameId::Valid(Cow::Borrowed("TDRC"))).unwrap();
		let Frame::Timestamp(TimestampFrame { timestamp, .. }) = tf else {
			unreachable!()
		};
		assert_eq!(timestamp.to_string(), "2012-04-17T12:01");

		let tf = id3v2.get(&FrameId::Valid(Cow::Borrowed("TIPL"))).unwrap();
		let Frame::KeyValue(KeyValueFrame {
			key_value_pairs, ..
		}) = tf
		else {
			unreachable!()
		};
		assert_eq!(key_value_pairs.len(), 4);
		assert_eq!(
			key_value_pairs[0],
			(Cow::Borrowed("Guitar"), Cow::Borrowed("Artist 1"))
		);
		assert_eq!(
			key_value_pairs[1],
			(Cow::Borrowed("Drums"), Cow::Borrowed("Artist 2"))
		);
		assert_eq!(
			key_value_pairs[2],
			(Cow::Borrowed("Producer"), Cow::Borrowed("Artist 3"))
		);
		assert_eq!(
			key_value_pairs[3],
			(Cow::Borrowed("Mastering"), Cow::Borrowed("Artist 4"))
		);

		// NOTE: Lofty upgrades the first genre (originally 51) to "Techno-Industrial"
		//       TagLib retains the original genre index.
		let tf = id3v2.genres().unwrap().collect::<Vec<_>>();
		assert_eq!(tf.join("\0"), "Techno-Industrial\0Noise\0Power Noise");

		assert!(!id3v2.contains(&FrameId::Valid(Cow::Borrowed("TDRL"))));
		assert!(!id3v2.contains(&FrameId::Valid(Cow::Borrowed("TDTG"))));
		assert!(!id3v2.contains(&FrameId::Valid(Cow::Borrowed("TMOO"))));
		assert!(!id3v2.contains(&FrameId::Valid(Cow::Borrowed("TPRO"))));
		assert!(!id3v2.contains(&FrameId::Valid(Cow::Borrowed("TSOA"))));
		assert!(!id3v2.contains(&FrameId::Valid(Cow::Borrowed("TSOT"))));
		assert!(!id3v2.contains(&FrameId::Valid(Cow::Borrowed("TSST"))));
		assert!(!id3v2.contains(&FrameId::Valid(Cow::Borrowed("TSOP"))));
	}
	file.rewind().unwrap();
	{
		#[allow(clippy::items_after_statements)]
		const EXPECTED_ID3V23_DATA: &[u8] = b"ID3\x03\x00\x00\x00\x00\x09\x28\
	TORY\x00\x00\x00\x05\x00\x00\x002011\
	TYER\x00\x00\x00\x05\x00\x00\x002012\
	TDAT\x00\x00\x00\x05\x00\x00\x001704\
	TIME\x00\x00\x00\x05\x00\x00\x001201\
	TCON\x00\x00\x00\x14\x00\x00\x00(51)(39)Power Noise\
	IPLS\x00\x00\x00\x44\x00\x00\x00Guitar\x00\
	Artist 1\x00Drums\x00Artist 2\x00Producer\x00\
	Artist 3\x00Mastering\x00Artist 4";

		let mut file_id3v2 = vec![0; EXPECTED_ID3V23_DATA.len()];
		file.read_exact(&mut file_id3v2).unwrap();
		assert_eq!(file_id3v2.as_slice(), EXPECTED_ID3V23_DATA);
	}
	{
		let mut file = temp_file!("tests/taglib/data/rare_frames.mp3");
		let f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		assert!(f.id3v2().is_some());
		file.rewind().unwrap();
		f.save_to(&mut file, WriteOptions::new().use_id3v23(true))
			.unwrap();

		file.rewind().unwrap();
		let mut file_content = Vec::new();
		file.read_to_end(&mut file_content).unwrap();

		let tcon_pos = file_content.windows(4).position(|w| w == b"TCON").unwrap();
		let tcon = &file_content[tcon_pos + 11..];
		assert_eq!(&tcon[..4], &b"(13)"[..]);
	}
}

#[test_log::test]
fn test_compressed_frame_with_broken_length() {
	let mut file = temp_file!("tests/taglib/data/compressed_id3_frame.mp3");
	let f = MpegFile::read_from(&mut file, ParseOptions::new().read_properties(false)).unwrap();
	assert!(
		f.id3v2()
			.unwrap()
			.contains(&FrameId::Valid(Cow::from("APIC")))
	);

	let frame = f
		.id3v2()
		.unwrap()
		.get(&FrameId::Valid(Cow::Borrowed("APIC")))
		.unwrap();
	let Frame::Picture(AttachedPictureFrame { picture, .. }) = frame else {
		unreachable!()
	};

	assert_eq!(picture.mime_type(), Some(&MimeType::Bmp));
	assert_eq!(picture.pic_type(), PictureType::Other);
	assert!(picture.description().is_none());
	assert_eq!(picture.data().len(), 86414);
}

#[test_log::test]
fn test_w000() {
	let mut file = temp_file!("tests/taglib/data/w000.mp3");
	let f = MpegFile::read_from(&mut file, ParseOptions::new().read_properties(false)).unwrap();

	assert!(
		f.id3v2()
			.unwrap()
			.contains(&FrameId::Valid(Cow::from("W000")))
	);
	let frame = f
		.id3v2()
		.unwrap()
		.get(&FrameId::Valid(Cow::Borrowed("W000")))
		.unwrap();
	let Frame::Url(url_frame) = frame else {
		unreachable!()
	};
	assert_eq!(url_frame.url(), "lukas.lalinsky@example.com____");
}

#[test_log::test]
#[ignore = "Marker test, Lofty does not replicate the property interface"]
fn test_property_interface() {}

#[test_log::test]
#[ignore = "Marker test, Lofty does not replicate the property interface"]
fn test_property_interface2() {}

#[test_log::test]
#[ignore = "Marker test, Lofty does not replicate the property interface"]
fn test_properties_movement() {
	// Outside of that, this is simply a text frame parsing test, which is redundant.
}

#[test_log::test]
#[ignore = "Marker test, Lofty does not replicate the property interface"]
fn test_property_grouping() {
	// Outside of that, this is simply a text frame parsing test, which is redundant.
}

#[test_log::test]
fn test_delete_frame() {
	let mut file = temp_file!("tests/taglib/data/rare_frames.mp3");

	{
		let mut f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		let t = f.id3v2_mut().unwrap();
		let _ = t.remove(&FrameId::Valid(Cow::Borrowed("TCON")));
		f.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let f2 = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		let t = f2.id3v2().unwrap();
		assert!(!t.contains(&FrameId::Valid(Cow::from("TCON"))));
	}
}

#[test_log::test]
fn test_save_and_strip_id3v1_should_not_add_frame_from_id3v1_to_id3v2() {
	let mut file = temp_file!("tests/taglib/data/xing.mp3");

	{
		let mut foo = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		let mut tag = Id3v2Tag::new();
		tag.set_artist(String::from("Artist"));
		foo.set_id3v2(tag);
		foo.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();
	{
		let mut bar = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		let _ = bar
			.id3v2_mut()
			.unwrap()
			.remove(&FrameId::Valid(Cow::Borrowed("TPE1")));

		bar.save_to(&mut file, WriteOptions::default()).unwrap();
	}
	file.rewind().unwrap();

	let f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
	assert!(f.id3v2().is_none());
}

// TODO: Support CHAP frames (#189)
#[test_log::test]
#[ignore = "CHAP frames aren't support yet"]
fn test_parse_chapter_frame() {}

// TODO: Support CHAP frames (#189)
#[test_log::test]
#[ignore = "CHAP frames aren't support yet"]
fn test_render_chapter_frame() {}

// TODO: Support CTOC frames (#189)
#[test_log::test]
#[ignore = "CTOC frames aren't support yet"]
fn test_parse_table_of_contents_frame() {}

// TODO: Support CTOC frames (#189)
#[test_log::test]
#[ignore = "CTOC frames aren't support yet"]
fn test_render_table_of_contents_frame() {}

#[test_log::test]
#[ignore = "Marker test, Lofty will not remove empty frames, as they can be valid"]
fn test_empty_frame() {}

#[test_log::test]
#[ignore = "Marker test, Lofty will combine duplicated tags"]
fn test_duplicate_tags() {}

// TODO: Support CTOC frames (#189)
#[test_log::test]
#[ignore = "CTOC frames aren't support yet"]
fn test_parse_toc_frame_with_many_children() {}
