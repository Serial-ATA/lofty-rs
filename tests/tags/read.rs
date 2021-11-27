use lofty::ape::{ApeItem, ApeTag};
use lofty::id3::v1::Id3v1Tag;
use lofty::id3::v2::{Frame, FrameFlags, FrameValue, Id3v2Tag, LanguageFrame, TextEncoding};
use lofty::iff::RiffInfoList;
use lofty::mp4::{Atom, AtomData, AtomIdent, Ilst};
use lofty::ogg::VorbisComments;
use lofty::ItemValue;

const APE: [u8; 209] = *include_bytes!("assets/test.apev2");
const ID3V1: [u8; 128] = *include_bytes!("assets/test.id3v1");
const ID3V2: [u8; 1168] = *include_bytes!("assets/test.id3v2");
const ILST: [u8; 1024] = *include_bytes!("assets/test.ilst");
const RIFF_INFO: [u8; 100] = *include_bytes!("assets/test.riff");
const VORBIS_COMMENTS: [u8; 152] = *include_bytes!("assets/test.vorbis");

#[test]
fn read_ape() {
	let mut expected_tag = ApeTag::default();

	let title_item = ApeItem::new(
		String::from("TITLE"),
		ItemValue::Text(String::from("Foo title")),
	)
	.unwrap();

	let artist_item = ApeItem::new(
		String::from("ARTIST"),
		ItemValue::Text(String::from("Bar artist")),
	)
	.unwrap();

	let album_item = ApeItem::new(
		String::from("ALBUM"),
		ItemValue::Text(String::from("Baz album")),
	)
	.unwrap();

	let comment_item = ApeItem::new(
		String::from("COMMENT"),
		ItemValue::Text(String::from("Qux comment")),
	)
	.unwrap();

	let year_item =
		ApeItem::new(String::from("YEAR"), ItemValue::Text(String::from("1984"))).unwrap();

	let track_number_item =
		ApeItem::new(String::from("TRACK"), ItemValue::Text(String::from("1"))).unwrap();

	let genre_item = ApeItem::new(
		String::from("GENRE"),
		ItemValue::Text(String::from("Classical")),
	)
	.unwrap();

	expected_tag.push_item(title_item);
	expected_tag.push_item(artist_item);
	expected_tag.push_item(album_item);
	expected_tag.push_item(comment_item);
	expected_tag.push_item(year_item);
	expected_tag.push_item(track_number_item);
	expected_tag.push_item(genre_item);

	let parsed_tag = ApeTag::read_from(&mut std::io::Cursor::new(APE)).unwrap();

	assert_eq!(expected_tag, parsed_tag);
}

#[test]
fn read_id3v1() {
	let expected_tag = Id3v1Tag {
		title: Some(String::from("Foo title")),
		artist: Some(String::from("Bar artist")),
		album: Some(String::from("Baz album")),
		year: Some(String::from("1984")),
		comment: Some(String::from("Qux comment")),
		track_number: Some(1),
		genre: Some(32),
	};

	let parsed_tag = Id3v1Tag::read_from(ID3V1);

	assert_eq!(expected_tag, parsed_tag);
}

#[test]
fn read_id3v2() {
	let mut expected_tag = Id3v2Tag::default();

	let encoding = TextEncoding::Latin1;
	let flags = FrameFlags::default();

	expected_tag.insert(
		Frame::new(
			"TPE1",
			FrameValue::Text {
				encoding,
				value: String::from("Bar artist"),
			},
			flags,
		)
		.unwrap(),
	);

	expected_tag.insert(
		Frame::new(
			"TIT2",
			FrameValue::Text {
				encoding,
				value: String::from("Foo title"),
			},
			flags,
		)
		.unwrap(),
	);

	expected_tag.insert(
		Frame::new(
			"TALB",
			FrameValue::Text {
				encoding,
				value: String::from("Baz album"),
			},
			flags,
		)
		.unwrap(),
	);

	expected_tag.insert(
		Frame::new(
			"COMM",
			FrameValue::Comment(LanguageFrame {
				encoding,
				language: String::from("eng"),
				description: String::new(),
				content: String::from("Qux comment"),
			}),
			flags,
		)
		.unwrap(),
	);

	expected_tag.insert(
		Frame::new(
			"TDRC",
			FrameValue::Text {
				encoding,
				value: String::from("1984"),
			},
			flags,
		)
		.unwrap(),
	);

	expected_tag.insert(
		Frame::new(
			"TRCK",
			FrameValue::Text {
				encoding,
				value: String::from("1"),
			},
			flags,
		)
		.unwrap(),
	);

	expected_tag.insert(
		Frame::new(
			"TCON",
			FrameValue::Text {
				encoding,
				value: String::from("Classical"),
			},
			flags,
		)
		.unwrap(),
	);

	let parsed_tag = Id3v2Tag::read_from(&mut &ID3V2[..]).unwrap();

	assert_eq!(expected_tag, parsed_tag);
}

#[test]
fn read_mp4_ilst() {
	let mut expected_tag = Ilst::default();

	// The track number is stored with a code 0,
	// meaning the there is no need to indicate the type,
	// which is `u64` in this case
	expected_tag.insert_atom(Atom::new(
		AtomIdent::Fourcc(*b"trkn"),
		AtomData::Unknown {
			code: 0,
			data: vec![0, 0, 0, 1, 0, 0, 0, 0],
		},
	));

	expected_tag.insert_atom(Atom::new(
		AtomIdent::Fourcc(*b"\xa9ART"),
		AtomData::UTF8(String::from("Bar artist")),
	));

	expected_tag.insert_atom(Atom::new(
		AtomIdent::Fourcc(*b"\xa9alb"),
		AtomData::UTF8(String::from("Baz album")),
	));

	expected_tag.insert_atom(Atom::new(
		AtomIdent::Fourcc(*b"\xa9cmt"),
		AtomData::UTF8(String::from("Qux comment")),
	));

	expected_tag.insert_atom(Atom::new(
		AtomIdent::Fourcc(*b"\xa9day"),
		AtomData::UTF8(String::from("1984")),
	));

	expected_tag.insert_atom(Atom::new(
		AtomIdent::Fourcc(*b"\xa9gen"),
		AtomData::UTF8(String::from("Classical")),
	));

	expected_tag.insert_atom(Atom::new(
		AtomIdent::Fourcc(*b"\xa9nam"),
		AtomData::UTF8(String::from("Foo title")),
	));

	let parsed_tag = Ilst::read_from(&mut &ILST[..], ILST.len() as u64).unwrap();

	assert_eq!(expected_tag, parsed_tag);
}

#[test]
fn read_riff_info() {
	let mut expected_tag = RiffInfoList::default();

	expected_tag.insert(String::from("IART"), String::from("Bar artist"));
	expected_tag.insert(String::from("ICMT"), String::from("Qux comment"));
	expected_tag.insert(String::from("ICRD"), String::from("1984"));
	expected_tag.insert(String::from("INAM"), String::from("Foo title"));
	expected_tag.insert(String::from("IPRD"), String::from("Baz album"));
	expected_tag.insert(String::from("IPRT"), String::from("1"));

	let mut reader = std::io::Cursor::new(&RIFF_INFO[..]);
	let parsed_tag = RiffInfoList::read_from(&mut reader, (RIFF_INFO.len() - 1) as u64).unwrap();

	assert_eq!(expected_tag, parsed_tag);
}

#[test]
fn read_vorbis_comments() {
	let mut expected_tag = VorbisComments::default();

	expected_tag.set_vendor(String::from("Lavf58.76.100"));

	expected_tag.insert_item(String::from("ALBUM"), String::from("Baz album"), false);
	expected_tag.insert_item(String::from("ARTIST"), String::from("Bar artist"), false);
	expected_tag.insert_item(String::from("COMMENT"), String::from("Qux comment"), false);
	expected_tag.insert_item(String::from("DATE"), String::from("1984"), false);
	expected_tag.insert_item(String::from("GENRE"), String::from("Classical"), false);
	expected_tag.insert_item(String::from("TITLE"), String::from("Foo title"), false);
	expected_tag.insert_item(String::from("TRACKNUMBER"), String::from("1"), false);

	let parsed_tag = VorbisComments::read_from(&mut &VORBIS_COMMENTS[..]).unwrap();

	assert_eq!(expected_tag, parsed_tag);
}
