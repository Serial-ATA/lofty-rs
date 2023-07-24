use crate::temp_file;

use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Seek;

use lofty::id3::v2::{
	AttachedPictureFrame, ChannelInformation, ChannelType, CommentFrame, Event,
	EventTimingCodesFrame, EventType, ExtendedTextFrame, ExtendedUrlFrame, Frame, FrameFlags,
	FrameId, FrameValue, GeneralEncapsulatedObject, Id3v2Tag, Id3v2Version, OwnershipFrame,
	Popularimeter, PrivateFrame, RelativeVolumeAdjustmentFrame, SyncTextContentType,
	SynchronizedText, TimestampFormat, UniqueFileIdentifierFrame, UrlLinkFrame,
};
use lofty::mpeg::MpegFile;
use lofty::{
	Accessor, AudioFile, MimeType, ParseOptions, ParsingMode, Picture, PictureType, TagExt,
	TextEncoding,
};

#[test]
fn test_unsynch_decode() {
	let mut file = temp_file!("tests/taglib/data/unsynch.id3");
	let f = MpegFile::read_from(&mut file, ParseOptions::new().read_properties(false)).unwrap();

	assert!(f.id3v2().is_some());
	assert_eq!(
		f.id3v2().unwrap().title().as_deref(),
		Some("My babe just cares for me")
	);
}

#[test]
#[ignore] // TODO: We don't support downgrading 2.4 tags to 2.3
fn test_downgrade_utf8_for_id3v23_1() {}

#[test]
#[ignore] // TODO: We don't support downgrading 2.4 tags to 2.3
fn test_downgrade_utf8_for_id3v23_2() {}

#[test]
#[ignore] // TODO: Need to think of a nice way to handle multiple UTF-16 values separated by null
fn test_utf16be_delimiter() {}

#[test]
#[ignore] // TODO: Need to think of a nice way to handle multiple UTF-16 values separated by null
fn test_utf16_delimiter() {}

#[test]
fn test_broken_frame1() {
	let mut file = temp_file!("tests/taglib/data/broken-tenc.id3");
	let f = MpegFile::read_from(&mut file, ParseOptions::new().read_properties(false)).unwrap();

	assert!(f
		.id3v2()
		.unwrap()
		.contains(&FrameId::Valid(Cow::from("TENC"))));
}

#[test]
#[ignore]
fn test_read_string_field() {
	// Marker test, this is not an API Lofty replicates
}

#[test]
fn test_parse_apic() {
	let f = AttachedPictureFrame::parse(
		&mut &b"\
	\x00\
	m\x00\
	\x01\
	d\x00\
	\x00"[..],
		Id3v2Version::V4,
	)
	.unwrap();
	assert_eq!(f.picture.mime_type(), &MimeType::Unknown(String::from("m")));
	assert_eq!(f.picture.pic_type(), PictureType::Icon);
	assert_eq!(f.picture.description(), Some("d"));
}

#[test]
fn test_parse_apic_utf16_bom() {
	let f = AttachedPictureFrame::parse(
		&mut &b"\
	\x01\x69\x6d\x61\x67\x65\
	\x2f\x6a\x70\x65\x67\x00\x00\xfe\xff\x00\x63\x00\x6f\x00\x76\x00\
	\x65\x00\x72\x00\x2e\x00\x6a\x00\x70\x00\x67\x00\x00\xff\xd8\xff"[..],
		Id3v2Version::V4,
	)
	.unwrap();

	assert_eq!(f.picture.mime_type(), &MimeType::Jpeg);
	assert_eq!(f.picture.pic_type(), PictureType::Other);
	assert_eq!(f.picture.description(), Some("cover.jpg"));
	assert_eq!(f.picture.data(), b"\xff\xd8\xff");
}

#[test]
fn test_parse_apicv22() {
	let frame = AttachedPictureFrame::parse(
		&mut &b"\
	\x00\
	JPG\
	\x01\
	d\x00\
	\x00"[..],
		Id3v2Version::V2,
	)
	.unwrap();

	assert_eq!(frame.picture.mime_type(), &MimeType::Jpeg);
	assert_eq!(frame.picture.pic_type(), PictureType::Icon);
	assert_eq!(frame.picture.description(), Some("d"));
}

#[test]
fn test_render_apic() {
	let f = AttachedPictureFrame {
		encoding: TextEncoding::UTF8,
		picture: Picture::new_unchecked(
			PictureType::CoverBack,
			MimeType::Png,
			Some(String::from("Description")),
			b"PNG data".to_vec(),
		),
	};

	assert_eq!(
		f.as_bytes(Id3v2Version::V4).unwrap(),
		b"\
	\x03\
	image/png\x00\
	\x04\
	Description\x00\
	PNG data"
	);
}

#[test]
#[ignore]
fn test_dont_render22() {
	// Marker test, not sure what's going on here?
}

#[test]
fn test_parse_geob() {
	let f = GeneralEncapsulatedObject::parse(
		b"\
	\x00\
	m\x00\
	f\x00\
	d\x00\
	\x00",
	)
	.unwrap();
	assert_eq!(f.mime_type.as_deref(), Some("m"));
	assert_eq!(f.file_name.as_deref(), Some("f"));
	assert_eq!(f.descriptor.as_deref(), Some("d"));
}

#[test]
fn test_render_geob() {
	let f = GeneralEncapsulatedObject {
		encoding: TextEncoding::Latin1,
		mime_type: Some(String::from("application/octet-stream")),
		file_name: Some(String::from("test.bin")),
		descriptor: Some(String::from("Description")),
		data: vec![0x01; 3],
	};

	assert_eq!(
		f.as_bytes(),
		b"\
	\x00\
	application/octet-stream\x00\
	test.bin\x00\
	Description\x00\
	\x01\x01\x01"
	);
}

#[test]
fn test_parse_popm() {
	let f = Popularimeter::parse(
		&mut &b"\
	email@example.com\x00\
	\x02\
	\x00\x00\x00\x03"[..],
	)
	.unwrap();
	assert_eq!(f.email, "email@example.com");
	assert_eq!(f.rating, 2);
	assert_eq!(f.counter, 3);
}

#[test]
fn test_parse_popm_without_counter() {
	let f = Popularimeter::parse(
		&mut &b"\
	email@example.com\x00\
	\x02"[..],
	)
	.unwrap();
	assert_eq!(f.email, "email@example.com");
	assert_eq!(f.rating, 2);
	assert_eq!(f.counter, 0);
}

#[test]
fn test_render_popm() {
	let f = Popularimeter {
		email: String::from("email@example.com"),
		rating: 2,
		counter: 3,
	};

	assert_eq!(
		f.as_bytes(),
		b"\
	email@example.com\x00\
	\x02\
	\x00\x00\x00\x03"
	);
}

#[test]
#[ignore]
fn test_popm_to_string() {
	// Marker test, Lofty doesn't have a display impl for Popularimeter
}

#[test]
fn test_popm_from_file() {
	let mut file = temp_file!("tests/taglib/data/xing.mp3");

	let f = Popularimeter {
		email: String::from("email@example.com"),
		rating: 200,
		counter: 3,
	};

	{
		let mut foo = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		let mut tag = Id3v2Tag::new();
		tag.insert(
			Frame::new("POPM", FrameValue::Popularimeter(f), FrameFlags::default()).unwrap(),
		);
		foo.set_id3v2(tag);
		foo.save_to(&mut file).unwrap();
	}
	file.rewind().unwrap();
	{
		let bar = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		let popm_frame = bar.id3v2().unwrap().get("POPM").unwrap();
		let popularimeter = match popm_frame.content() {
			FrameValue::Popularimeter(popm) => popm,
			_ => unreachable!(),
		};

		assert_eq!(popularimeter.email, "email@example.com");
		assert_eq!(popularimeter.rating, 200);
	}
}

#[test]
fn test_parse_relative_volume_frame() {
	let f = RelativeVolumeAdjustmentFrame::parse(
		&mut &b"\
	ident\x00\
    \x02\
    \x00\x0F\
    \x08\
    \x45"[..],
		ParsingMode::Strict,
	)
	.unwrap()
	.unwrap();

	assert_eq!(f.identification, "ident");
	let front_right = f.channels.get(&ChannelType::FrontRight).unwrap();
	assert_eq!(
		front_right.volume_adjustment as f32 / 512.0f32,
		15.0f32 / 512.0f32
	);
	assert_eq!(front_right.volume_adjustment, 15);
	assert_eq!(front_right.bits_representing_peak, 8);
	assert_eq!(front_right.peak_volume, Some(vec![0x45]));
	let channels = f.channels;
	assert_eq!(channels.len(), 1);
}

#[test]
fn test_render_relative_volume_frame() {
	let f = RelativeVolumeAdjustmentFrame {
		identification: String::from("ident"),
		channels: {
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
			m
		},
	};

	assert_eq!(
		f.as_bytes(),
		b"\
	ident\x00\
    \x02\
    \x00\x0F\
    \x08\
    \x45"
	);
}

#[test]
fn test_parse_unique_file_identifier_frame() {
	let f = UniqueFileIdentifierFrame::parse(
		&mut &b"\
	owner\x00\
	\x00\x01\x02"[..],
		ParsingMode::Strict,
	)
	.unwrap()
	.unwrap();

	assert_eq!(f.owner, "owner");
	assert_eq!(f.identifier, &[0x00, 0x01, 0x02]);
}

#[test]
fn test_parse_empty_unique_file_identifier_frame() {
	let f = UniqueFileIdentifierFrame::parse(
		&mut &b"\
	\x00\
	"[..],
		ParsingMode::Strict,
	);

	// NOTE: TagLib considers a missing owner to be valid, we do not
	assert!(f.is_err());
}

#[test]
fn test_render_unique_file_identifier_frame() {
	let f = UniqueFileIdentifierFrame {
		owner: String::from("owner"),
		identifier: b"\x01\x02\x03".to_vec(),
	};

	assert_eq!(
		f.as_bytes(),
		b"\
owner\x00\
\x01\x02\x03"
	);
}

#[test]
fn test_parse_url_link_frame() {
	let f = UrlLinkFrame::parse(&mut &b"http://example.com"[..])
		.unwrap()
		.unwrap();
	assert_eq!(f.url(), "http://example.com");
}

#[test]
fn test_render_url_link_frame() {
	let f = UrlLinkFrame::parse(&mut &b"http://example.com"[..])
		.unwrap()
		.unwrap();
	assert_eq!(f.as_bytes(), b"http://example.com");
}

#[test]
fn test_parse_user_url_link_frame() {
	let f = ExtendedUrlFrame::parse(
		&mut &b"\
	\x00\
	foo\x00\
	http://example.com"[..],
		Id3v2Version::V4,
	)
	.unwrap()
	.unwrap();

	assert_eq!(f.description, String::from("foo"));
	assert_eq!(f.content, String::from("http://example.com"));
}

#[test]
fn test_render_user_url_link_frame() {
	let f = ExtendedUrlFrame {
		encoding: TextEncoding::Latin1,
		description: String::from("foo"),
		content: String::from("http://example.com"),
	};

	assert_eq!(
		f.as_bytes(),
		b"\
	\x00\
	foo\x00\
	http://example.com"
	);
}

#[test]
fn test_parse_ownership_frame() {
	let f = OwnershipFrame::parse(
		&mut &b"\
		\x00\
        GBP1.99\x00\
		20120905\
		Beatport"[..],
	)
	.unwrap()
	.unwrap();

	assert_eq!(f.price_paid, "GBP1.99");
	assert_eq!(f.date_of_purchase, "20120905");
	assert_eq!(f.seller, "Beatport");
}

#[test]
fn test_render_ownership_frame() {
	let f = OwnershipFrame {
		encoding: TextEncoding::Latin1,
		price_paid: String::from("GBP1.99"),
		date_of_purchase: String::from("20120905"),
		seller: String::from("Beatport"),
	};

	assert_eq!(
		f.as_bytes().unwrap(),
		b"\
		\x00\
        GBP1.99\x00\
		20120905\
		Beatport"[..]
	)
}

#[test]
fn test_parse_synchronized_lyrics_frame() {
	let f = SynchronizedText::parse(
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

#[test]
fn test_parse_synchronized_lyrics_frame_with_empty_description() {
	let f = SynchronizedText::parse(
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

#[test]
fn test_render_synchronized_lyrics_frame() {
	let f = SynchronizedText {
		encoding: TextEncoding::Latin1,
		language: *b"eng",
		timestamp_format: TimestampFormat::MS,
		content_type: SyncTextContentType::Lyrics,
		description: Some(String::from("foo")),
		content: vec![
			(1234, String::from("Example")),
			(4567, String::from("Lyrics")),
		],
	};

	assert_eq!(
		f.as_bytes().unwrap(),
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

#[test]
fn test_parse_event_timing_codes_frame() {
	let f = EventTimingCodesFrame::parse(
		&mut &b"\
	\x02\
	\x02\
	\x00\x00\xf3\x5c\
	\xfe\
	\x00\x36\xee\x80"[..],
	)
	.unwrap()
	.unwrap();

	assert_eq!(f.timestamp_format, TimestampFormat::MS);

	let sel = f.events;
	assert_eq!(sel.len(), 2);
	assert_eq!(sel[0].event_type, EventType::IntroStart);
	assert_eq!(sel[0].timestamp, 62300);
	assert_eq!(sel[1].event_type, EventType::AudioFileEnds);
	assert_eq!(sel[1].timestamp, 3600000);
}

#[test]
fn test_render_event_timing_codes_frame() {
	let f = EventTimingCodesFrame {
		timestamp_format: TimestampFormat::MS,
		events: vec![
			Event {
				event_type: EventType::IntroStart,
				timestamp: 62300,
			},
			Event {
				event_type: EventType::AudioFileEnds,
				timestamp: 3600000,
			},
		],
	};

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

#[test]
fn test_parse_comments_frame() {
	let f = CommentFrame::parse(
		&mut &b"\x03\
								deu\
								Description\x00\
								Text"[..],
		Id3v2Version::V4,
	)
	.unwrap()
	.unwrap();

	assert_eq!(f.encoding, TextEncoding::UTF8);
	assert_eq!(f.language, *b"deu");
	assert_eq!(f.description, String::from("Description"));
	assert_eq!(f.content, String::from("Text"));
}

#[test]
fn test_render_comments_frame() {
	let f = CommentFrame {
		encoding: TextEncoding::UTF16,
		language: *b"eng",
		description: String::from("Description"),
		content: String::from("Text"),
	};

	assert_eq!(
		f.as_bytes().unwrap(),
		b"\
	\x01\
	eng\
	\xff\xfeD\0e\0s\0c\0r\0i\0p\0t\0i\0o\0n\0\x00\x00\
	\xff\xfeT\0e\0x\0t\0"
	);
}

#[test]
#[ignore]
fn test_parse_podcast_frame() {
	// Marker test, Lofty doesn't have dedicated support for PCST frames, it seems unnecessary
}

#[test]
#[ignore]
fn test_render_podcast_frame() {
	// Marker test, Lofty doesn't have dedicated support for PCST frames, it seems unnecessary
}

#[test]
fn test_parse_private_frame() {
	let f = PrivateFrame::parse(
		&mut &b"\
	WM/Provider\x00\
	TL"[..],
	)
	.unwrap()
	.unwrap();

	assert_eq!(f.owner, "WM/Provider");
	assert_eq!(f.private_data, b"TL");
}

#[test]
fn test_render_private_frame() {
	let f = PrivateFrame {
		owner: String::from("WM/Provider"),
		private_data: b"TL".to_vec(),
	};

	assert_eq!(
		f.as_bytes(),
		b"\
	WM/Provider\x00\
	TL"
	);
}

#[test]
fn test_parse_user_text_identification_frame() {
	let frame_without_description = ExtendedUrlFrame::parse(
		&mut &b"\
	\x00\
	\x00\
	Text"[..],
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

#[test]
fn test_render_user_text_identification_frame() {
	let mut f = ExtendedTextFrame {
		encoding: TextEncoding::Latin1,
		description: String::new(),
		content: String::from("Text"),
	};

	assert_eq!(
		f.as_bytes(),
		b"\
	\x00\
	\x00\
	Text"
	);

	f.description = String::from("Description");

	assert_eq!(
		f.as_bytes(),
		b"\
	\x00\
	Description\x00\
	Text"
	);
}

#[test]
#[ignore] // TODO: iTunes, being the great application it is writes unsynchronized integers for sizes. There's no *great* way to detect this.
fn test_itunes_24_frame_size() {
	let mut file = temp_file!("tests/taglib/data/005411.id3");
	let f = MpegFile::read_from(&mut file, ParseOptions::new().read_properties(false)).unwrap();

	assert!(f
		.id3v2()
		.unwrap()
		.contains(&FrameId::Valid(Cow::from("TIT2"))));
	assert_eq!(
		f.id3v2().unwrap().get_text("TIT2").unwrap(),
		"Sunshine Superman"
	);
}

#[test]
fn test_save_utf16_comment() {
	let mut file = temp_file!("tests/taglib/data/xing.mp3");

	// NOTE: You can change the default encoding in TagLib, Lofty does not support this
	{
		let mut foo = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		let mut tag = Id3v2Tag::new();
		tag.insert(
			Frame::new(
				"COMM",
				CommentFrame {
					encoding: TextEncoding::UTF16,
					language: *b"eng",
					description: String::new(),
					content: String::from("Test comment!"),
				},
				FrameFlags::default(),
			)
			.unwrap(),
		);
		foo.set_id3v2(tag);
		foo.save_to(&mut file).unwrap();
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

#[test]
#[ignore] // TODO: We don't support downgrading to 2.3 tags yet
fn test_update_genre_23_1() {}

#[test]
#[ignore]
fn test_update_genre23_2() {
	// Marker test, Lofty doesn't do additional work with the genre string
}

#[test]
#[ignore]
fn test_update_genre23_3() {
	// Marker test, Lofty doesn't do additional work with the genre string
}

#[test]
#[ignore] // TODO: We currently just return the genre string as it is in the tag, need to think about whether or not to convert numerical strings
fn test_update_genre_24() {}

#[test]
fn test_update_date22() {
	let mut file = temp_file!("tests/taglib/data/id3v22-tda.mp3");
	let f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
	assert!(f.id3v2().is_some());
	assert_eq!(f.id3v2().unwrap().year(), Some(2010));
}

#[test]
#[ignore] // TODO: Determine if this is even worth doing. It is just combining TYE+TDA when upgrading ID3v2.2 to 2.4
fn test_update_full_date22() {
	let mut file = temp_file!("tests/taglib/data/id3v22-tda.mp3");
	let f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
	assert!(f.id3v2().is_some());
	assert_eq!(f.id3v2().unwrap().get_text("TDRC").unwrap(), "2010-04-03");
}

#[test]
#[ignore] // TODO: We don't support downgrading 2.4 tags to 2.3
fn test_downgrade_to_23() {}

#[test]
fn test_compressed_frame_with_broken_length() {
	let mut file = temp_file!("tests/taglib/data/compressed_id3_frame.mp3");
	let f = MpegFile::read_from(&mut file, ParseOptions::new().read_properties(false)).unwrap();
	assert!(f
		.id3v2()
		.unwrap()
		.contains(&FrameId::Valid(Cow::from("APIC"))));

	let frame = f.id3v2().unwrap().get("APIC").unwrap();
	let picture = match frame.content() {
		FrameValue::Picture(AttachedPictureFrame { picture, .. }) => picture,
		_ => unreachable!(),
	};

	assert_eq!(picture.mime_type(), &MimeType::Bmp);
	assert_eq!(picture.pic_type(), PictureType::Other);
	assert!(picture.description().is_none());
	assert_eq!(picture.data().len(), 86414);
}

#[test]
fn test_w000() {
	let mut file = temp_file!("tests/taglib/data/w000.mp3");
	let f = MpegFile::read_from(&mut file, ParseOptions::new().read_properties(false)).unwrap();

	assert!(f
		.id3v2()
		.unwrap()
		.contains(&FrameId::Valid(Cow::from("W000"))));
	let frame = f.id3v2().unwrap().get("W000").unwrap();
	let url_frame = match frame.content() {
		FrameValue::Url(url_frame) => url_frame,
		_ => unreachable!(),
	};
	assert_eq!(url_frame.url(), "lukas.lalinsky@example.com____");
}

#[test]
#[ignore]
fn test_property_interface() {
	// Marker test, Lofty does not replicate the property interface
}

#[test]
#[ignore]
fn test_property_interface2() {
	// Marker test, Lofty does not replicate the property interface
}

#[test]
#[ignore]
fn test_properties_movement() {
	// Marker test, Lofty does not replicate the property interface.
	// Outside of that, this is simply a text frame parsing test, which is redundant.
}

#[test]
#[ignore]
fn test_property_grouping() {
	// Marker test, Lofty does not replicate the property interface.
	// Outside of that, this is simply a text frame parsing test, which is redundant.
}

#[test]
fn test_delete_frame() {
	let mut file = temp_file!("tests/taglib/data/rare_frames.mp3");

	{
		let mut f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		let t = f.id3v2_mut().unwrap();
		let _ = t.remove(&FrameId::Valid(Cow::Borrowed("TCON")));
		f.save_to(&mut file).unwrap();
	}
	file.rewind().unwrap();
	{
		let f2 = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		let t = f2.id3v2().unwrap();
		assert!(!t.contains(&FrameId::Valid(Cow::from("TCON"))));
	}
}

#[test]
fn test_save_and_strip_id3v1_should_not_add_frame_from_id3v1_to_id3v2() {
	let mut file = temp_file!("tests/taglib/data/xing.mp3");

	{
		let mut foo = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		let mut tag = Id3v2Tag::new();
		tag.set_artist(String::from("Artist"));
		foo.set_id3v2(tag);
		foo.save_to(&mut file).unwrap();
	}
	file.rewind().unwrap();
	{
		let mut bar = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		let _ = bar
			.id3v2_mut()
			.unwrap()
			.remove(&FrameId::Valid(Cow::Borrowed("TPE1")));

		bar.save_to(&mut file).unwrap();
	}
	file.rewind().unwrap();

	let f = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
	assert!(f.id3v2().is_none());
}

#[test]
#[ignore] // TODO: We don't support CHAP frames yet
fn test_parse_chapter_frame() {}

#[test]
#[ignore] // TODO: We don't support CHAP frames yet
fn test_render_chapter_frame() {}

#[test]
#[ignore] // TODO: We don't support CTOC frames yet
fn test_parse_table_of_contents_frame() {}

#[test]
#[ignore] // TODO: We don't support CTOC frames yet
fn test_render_table_of_contents_frame() {}

#[test]
#[ignore]
fn test_empty_frame() {
	// Marker test, Lofty will not remove empty frames, as they can be valid
}

#[test]
#[ignore]
fn test_duplicate_tags() {
	// Marker test, Lofty will combine duplicated tags
}

#[test]
#[ignore] // TODO: We don't support CTOC frames yet
fn test_parse_toc_frame_with_many_children() {}
