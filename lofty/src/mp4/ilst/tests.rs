use crate::config::{ParseOptions, ParsingMode, WriteOptions};
use crate::mp4::ilst::TITLE;
use crate::mp4::ilst::atom::AtomDataStorage;
use crate::mp4::read::AtomReader;
use crate::mp4::{AdvisoryRating, Atom, AtomData, AtomIdent, DataType, Ilst, Mp4File};
use crate::picture::{MimeType, Picture, PictureType};
use crate::prelude::*;
use crate::tag::utils::test_utils;
use crate::tag::utils::test_utils::read_path;
use crate::tag::{ItemValue, Tag, TagItem, TagType};

use std::borrow::Cow;
use std::io::{Cursor, Read as _, Seek as _, Write as _};

fn read_ilst(path: &str, parse_options: ParseOptions) -> Ilst {
	let tag = std::fs::read(path).unwrap();
	read_ilst_raw(&tag, parse_options)
}

fn read_ilst_raw(bytes: &[u8], parse_options: ParseOptions) -> Ilst {
	read_ilst_with_options(bytes, parse_options)
}

fn read_ilst_strict(path: &str) -> Ilst {
	read_ilst(path, ParseOptions::new().parsing_mode(ParsingMode::Strict))
}

fn read_ilst_bestattempt(path: &str) -> Ilst {
	read_ilst(
		path,
		ParseOptions::new().parsing_mode(ParsingMode::BestAttempt),
	)
}

fn read_ilst_with_options(bytes: &[u8], parse_options: ParseOptions) -> Ilst {
	let len = bytes.len();

	let cursor = Cursor::new(bytes);
	let mut reader = AtomReader::new(cursor, parse_options.parsing_mode).unwrap();

	super::read::parse_ilst(&mut reader, parse_options, len as u64).unwrap()
}

fn verify_atom(ilst: &Ilst, ident: [u8; 4], data: &AtomData) {
	let atom = ilst.get(&AtomIdent::Fourcc(ident)).unwrap();
	assert_eq!(atom.data().next().unwrap(), data);
}

#[test_log::test]
fn parse_ilst() {
	let mut expected_tag = Ilst::default();

	// The track number is stored with a code 0,
	// meaning the there is no need to indicate the type,
	// which is `u64` in this case
	expected_tag.insert(Atom::new(
		AtomIdent::Fourcc(*b"trkn"),
		AtomData::Unknown {
			code: DataType::Reserved,
			data: vec![0, 0, 0, 1, 0, 0, 0, 0],
		},
	));

	// Same with disc numbers
	expected_tag.insert(Atom::new(
		AtomIdent::Fourcc(*b"disk"),
		AtomData::Unknown {
			code: DataType::Reserved,
			data: vec![0, 0, 0, 1, 0, 2],
		},
	));

	expected_tag.insert(Atom::new(
		AtomIdent::Fourcc(*b"\xa9ART"),
		AtomData::UTF8(String::from("Bar artist")),
	));

	expected_tag.insert(Atom::new(
		AtomIdent::Fourcc(*b"\xa9alb"),
		AtomData::UTF8(String::from("Baz album")),
	));

	expected_tag.insert(Atom::new(
		AtomIdent::Fourcc(*b"\xa9cmt"),
		AtomData::UTF8(String::from("Qux comment")),
	));

	expected_tag.insert(Atom::new(
		AtomIdent::Fourcc(*b"\xa9day"),
		AtomData::UTF8(String::from("1984")),
	));

	expected_tag.insert(Atom::new(
		AtomIdent::Fourcc(*b"\xa9gen"),
		AtomData::UTF8(String::from("Classical")),
	));

	expected_tag.insert(Atom::new(
		AtomIdent::Fourcc(*b"\xa9nam"),
		AtomData::UTF8(String::from("Foo title")),
	));

	let tag = crate::tag::utils::test_utils::read_path("tests/tags/assets/ilst/test.ilst");
	let len = tag.len();

	let cursor = Cursor::new(tag);
	let mut reader = AtomReader::new(cursor, ParsingMode::Strict).unwrap();

	let parsed_tag = super::read::parse_ilst(
		&mut reader,
		ParseOptions::new().parsing_mode(ParsingMode::Strict),
		len as u64,
	)
	.unwrap();

	assert_eq!(expected_tag, parsed_tag);
}

#[test_log::test]
fn ilst_re_read() {
	let parsed_tag = read_ilst_strict("tests/tags/assets/ilst/test.ilst");

	let mut writer = Vec::new();
	parsed_tag
		.dump_to(&mut writer, WriteOptions::default())
		.unwrap();

	let cursor = Cursor::new(&writer[8..]);
	let mut reader = AtomReader::new(cursor, ParsingMode::Strict).unwrap();

	// Remove the ilst identifier and size
	let temp_parsed_tag = super::read::parse_ilst(
		&mut reader,
		ParseOptions::new().parsing_mode(ParsingMode::Strict),
		(writer.len() - 8) as u64,
	)
	.unwrap();

	assert_eq!(parsed_tag, temp_parsed_tag);
}

#[test_log::test]
fn ilst_to_tag() {
	let tag = crate::tag::utils::test_utils::read_path("tests/tags/assets/ilst/test.ilst");
	let len = tag.len();

	let cursor = Cursor::new(tag);
	let mut reader = AtomReader::new(cursor, ParsingMode::Strict).unwrap();

	let ilst = super::read::parse_ilst(
		&mut reader,
		ParseOptions::new().parsing_mode(ParsingMode::Strict),
		len as u64,
	)
	.unwrap();

	let tag: Tag = ilst.into();

	crate::tag::utils::test_utils::verify_tag(&tag, true, true);

	assert_eq!(tag.get_string(ItemKey::DiscNumber), Some("1"));
	assert_eq!(tag.get_string(ItemKey::DiscTotal), Some("2"));
}

#[test_log::test]
fn tag_to_ilst() {
	let mut tag = crate::tag::utils::test_utils::create_tag(TagType::Mp4Ilst);

	tag.insert_text(ItemKey::DiscNumber, String::from("1"));
	tag.insert_text(ItemKey::DiscTotal, String::from("2"));

	let ilst: Ilst = tag.into();

	verify_atom(
		&ilst,
		*b"\xa9nam",
		&AtomData::UTF8(String::from("Foo title")),
	);
	verify_atom(
		&ilst,
		*b"\xa9ART",
		&AtomData::UTF8(String::from("Bar artist")),
	);
	verify_atom(
		&ilst,
		*b"\xa9alb",
		&AtomData::UTF8(String::from("Baz album")),
	);
	verify_atom(
		&ilst,
		*b"\xa9cmt",
		&AtomData::UTF8(String::from("Qux comment")),
	);
	verify_atom(
		&ilst,
		*b"\xa9gen",
		&AtomData::UTF8(String::from("Classical")),
	);
	verify_atom(
		&ilst,
		*b"trkn",
		&AtomData::Unknown {
			code: DataType::Reserved,
			data: vec![0, 0, 0, 1, 0, 0, 0, 0],
		},
	);
	verify_atom(
		&ilst,
		*b"disk",
		&AtomData::Unknown {
			code: DataType::Reserved,
			data: vec![0, 0, 0, 1, 0, 2, 0, 0],
		},
	)
}

#[test_log::test]
fn issue_34() {
	let ilst = read_ilst_strict("tests/tags/assets/ilst/issue_34.ilst");

	verify_atom(
		&ilst,
		*b"\xa9ART",
		&AtomData::UTF8(String::from("Foo artist")),
	);
	verify_atom(
		&ilst,
		*b"plID",
		&AtomData::Unknown {
			code: DataType::BeSignedInteger,
			data: 88888_u64.to_be_bytes().to_vec(),
		},
	)
}

#[test_log::test]
fn advisory_rating() {
	let ilst = read_ilst_strict("tests/tags/assets/ilst/advisory_rating.ilst");

	verify_atom(
		&ilst,
		*b"\xa9ART",
		&AtomData::UTF8(String::from("Foo artist")),
	);

	assert_eq!(ilst.advisory_rating(), Some(AdvisoryRating::Explicit));
}

#[test_log::test]
fn trailing_padding() {
	const ILST_START: usize = 97;
	const ILST_END: usize = 131;
	const PADDING_SIZE: usize = 990;

	let file_bytes = read_path("tests/files/assets/ilst_trailing_padding.m4a");
	assert!(
		Mp4File::read_from(
			&mut Cursor::new(&file_bytes),
			ParseOptions::new().read_properties(false)
		)
		.is_ok()
	);

	let mut ilst;
	let old_free_size;
	{
		let ilst_bytes = &file_bytes[ILST_START..ILST_END];

		old_free_size = u32::from_be_bytes(file_bytes[ILST_END..ILST_END + 4].try_into().unwrap());
		assert_eq!(old_free_size, PADDING_SIZE as u32);

		let cursor = Cursor::new(ilst_bytes);
		let mut reader = AtomReader::new(cursor, ParsingMode::Strict).unwrap();

		ilst = super::read::parse_ilst(
			&mut reader,
			ParseOptions::new().parsing_mode(ParsingMode::Strict),
			ilst_bytes.len() as u64,
		)
		.unwrap();
	}

	let mut file = tempfile::tempfile().unwrap();
	file.write_all(&file_bytes).unwrap();
	file.rewind().unwrap();

	ilst.set_title(String::from("Exactly 21 Characters"));
	ilst.save_to(&mut file, WriteOptions::default()).unwrap();

	// Now verify the free atom
	file.rewind().unwrap();

	let mut file_bytes = Vec::new();
	file.read_to_end(&mut file_bytes).unwrap();

	// 24 (atom + data) + title string (21)
	let new_data_size = 24_u32 + 21;
	let new_ilst_end = ILST_END + new_data_size as usize;

	let file_atom = &file_bytes[new_ilst_end..new_ilst_end + 8];

	match file_atom {
		[size @ .., b'f', b'r', b'e', b'e'] => assert_eq!(
			old_free_size - new_data_size,
			u32::from_be_bytes(size.try_into().unwrap())
		),
		_ => unreachable!(),
	}

	// Verify we can re-read the file
	file.rewind().unwrap();
	assert!(Mp4File::read_from(&mut file, ParseOptions::new().read_properties(false)).is_ok());
}

#[test_log::test]
fn read_non_full_meta_atom() {
	let file_bytes = read_path("tests/files/assets/non_full_meta_atom.m4a");
	let file = Mp4File::read_from(
		&mut Cursor::new(file_bytes),
		ParseOptions::new().read_properties(false),
	)
	.unwrap();

	assert!(file.ilst_tag.is_some());
}

#[test_log::test]
fn write_non_full_meta_atom() {
	// This is testing writing to a file with a non-full meta atom
	// We will *not* write a non-full meta atom

	let file_bytes = read_path("tests/files/assets/non_full_meta_atom.m4a");
	let mut file = tempfile::tempfile().unwrap();
	file.write_all(&file_bytes).unwrap();
	file.rewind().unwrap();

	let mut tag = Ilst::default();
	tag.insert(Atom {
		ident: AtomIdent::Fourcc(*b"\xa9ART"),
		data: AtomDataStorage::Single(AtomData::UTF8(String::from("Foo artist"))),
	});

	tag.save_to(&mut file, WriteOptions::default()).unwrap();
	file.rewind().unwrap();

	let mp4_file = Mp4File::read_from(&mut file, ParseOptions::new()).unwrap();
	assert!(mp4_file.ilst_tag.is_some());

	verify_atom(
		&mp4_file.ilst_tag.unwrap(),
		*b"\xa9ART",
		&AtomData::UTF8(String::from("Foo artist")),
	);
}

#[test_log::test]
fn multi_value_atom() {
	let ilst = read_ilst_strict("tests/tags/assets/ilst/multi_value_atom.ilst");
	let artist_atom = ilst.get(&AtomIdent::Fourcc(*b"\xa9ART")).unwrap();

	assert_eq!(
		artist_atom.data,
		AtomDataStorage::Multiple(vec![
			AtomData::UTF8(String::from("Foo artist")),
			AtomData::UTF8(String::from("Bar artist")),
		])
	);

	// Sanity single value atom
	verify_atom(
		&ilst,
		*b"\xa9gen",
		&AtomData::UTF8(String::from("Classical")),
	);
}

#[test_log::test]
fn multi_value_roundtrip() {
	let mut tag = Tag::new(TagType::Mp4Ilst);
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
	tag.insert_text(ItemKey::Comment, "Comment 1".to_owned());
	tag.push(TagItem::new(
		ItemKey::Comment,
		ItemValue::Text("Comment 2".to_owned()),
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
	assert_eq!(20, tag.len());

	let ilst = Ilst::from(tag.clone());
	let (split_remainder, split_tag) = ilst.split_tag();

	assert_eq!(0, split_remainder.len());
	assert_eq!(tag.len(), split_tag.len());
	assert_eq!(tag.items, split_tag.items);
}

#[test_log::test]
fn zero_sized_ilst() {
	let file = Mp4File::read_from(
		&mut Cursor::new(test_utils::read_path("tests/files/assets/zero/zero.ilst")),
		ParseOptions::new().read_properties(false),
	)
	.unwrap();

	assert_eq!(file.ilst(), Some(&Ilst::default()));
}

#[test_log::test]
fn merge_insert() {
	let mut ilst = Ilst::new();

	// Insert two titles
	ilst.set_title(String::from("Foo"));
	ilst.insert(Atom::new(TITLE, AtomData::UTF8(String::from("Bar"))));

	// Title should still be the first value, but there should be two total values
	assert_eq!(ilst.title().as_deref(), Some("Foo"));
	assert_eq!(ilst.get(&TITLE).unwrap().data().count(), 2);

	// Meaning we only have 1 atom
	assert_eq!(ilst.len(), 1);
}

#[test_log::test]
fn invalid_atom_type() {
	let ilst = read_ilst_strict("tests/tags/assets/ilst/invalid_atom_type.ilst");

	// The tag contains 3 items, however the last one has an invalid type. We will stop at that point, but retain the
	// first two items.
	assert_eq!(ilst.len(), 2);

	assert_eq!(ilst.track().unwrap(), 1);
	assert_eq!(ilst.track_total().unwrap(), 0);
	assert_eq!(ilst.disk().unwrap(), 1);
	assert_eq!(ilst.disk_total().unwrap(), 2);
}

#[test_log::test]
fn invalid_string_encoding() {
	let ilst = read_ilst_bestattempt("tests/tags/assets/ilst/invalid_string_encoding.ilst");

	// The tag has an album string with some unknown encoding, but the rest of the tag
	// is valid. We should have all items present except the album.
	assert_eq!(ilst.len(), 3);

	assert_eq!(ilst.artist().unwrap(), "Foo artist");
	assert_eq!(ilst.title().unwrap(), "Bar title");
	assert_eq!(ilst.comment().unwrap(), "Baz comment");

	assert!(ilst.album().is_none());
}

#[test_log::test]
fn flag_item_conversion() {
	let mut tag = Tag::new(TagType::Mp4Ilst);
	tag.insert_text(ItemKey::FlagCompilation, "1".to_owned());
	tag.insert_text(ItemKey::FlagPodcast, "0".to_owned());

	let ilst: Ilst = tag.into();
	assert_eq!(
		ilst.get(&AtomIdent::Fourcc(*b"cpil"))
			.unwrap()
			.data()
			.next()
			.unwrap(),
		&AtomData::Bool(true)
	);
	assert_eq!(
		ilst.get(&AtomIdent::Fourcc(*b"pcst"))
			.unwrap()
			.data()
			.next()
			.unwrap(),
		&AtomData::Bool(false)
	);
}

#[test_log::test]
fn special_items_roundtrip() {
	let mut tag = Ilst::new();

	let atom = Atom::new(
		AtomIdent::Fourcc(*b"SMTH"),
		AtomData::Unknown {
			code: DataType::Reserved,
			data: b"Meaningless Data".to_vec(),
		},
	);

	tag.insert(atom.clone());
	tag.set_artist(String::from("Foo Artist")); // Some value that we *can* represent generically

	let tag: Tag = tag.into();

	assert_eq!(tag.len(), 1);
	assert_eq!(tag.artist().as_deref(), Some("Foo Artist"));

	let tag: Ilst = tag.into();

	assert_eq!(tag.atoms.len(), 2);
	assert_eq!(tag.artist().as_deref(), Some("Foo Artist"));
	assert_eq!(tag.get(&AtomIdent::Fourcc(*b"SMTH")), Some(&atom));

	let mut tag_bytes = Vec::new();
	tag.dump_to(&mut tag_bytes, WriteOptions::default())
		.unwrap();

	tag_bytes.drain(..8); // Remove the ilst identifier and size for `read_ilst`

	let tag_re_read = read_ilst_raw(
		&tag_bytes[..],
		ParseOptions::new().parsing_mode(ParsingMode::Strict),
	);
	assert_eq!(tag, tag_re_read);

	// Now write from `Tag`
	let tag: Tag = tag.into();

	let mut tag_bytes = Vec::new();
	tag.dump_to(&mut tag_bytes, WriteOptions::default())
		.unwrap();

	tag_bytes.drain(..8); // Remove the ilst identifier and size for `read_ilst`

	let generic_tag_re_read = read_ilst_raw(
		&tag_bytes[..],
		ParseOptions::new().parsing_mode(ParsingMode::Strict),
	);
	assert_eq!(tag_re_read, generic_tag_re_read);
}

#[test_log::test]
fn skip_reading_cover_art() {
	let p = Picture::unchecked(std::iter::repeat_n(0, 50).collect::<Vec<u8>>())
		.pic_type(PictureType::CoverFront)
		.mime_type(MimeType::Jpeg)
		.build();

	let mut tag = Tag::new(TagType::Mp4Ilst);
	tag.push_picture(p);

	tag.set_artist(String::from("Foo artist"));

	let mut writer = Vec::new();
	tag.dump_to(&mut writer, WriteOptions::new()).unwrap();

	// Skip `ilst` header
	let ilst = read_ilst_with_options(&writer[8..], ParseOptions::new().read_cover_art(false));
	assert_eq!(ilst.len(), 1); // Artist, no picture
	assert!(ilst.artist().is_some());
}

#[test_log::test]
fn gnre_conversion_case_1() {
	// Case 1: 1 `gnre` atom present, no `©gen` present. `gnre` gets converted without issue.
	let ilst = read_ilst_bestattempt("tests/tags/assets/ilst/gnre_conversion_case_1.ilst");

	assert_eq!(ilst.len(), 2);
	assert_eq!(ilst.artist().unwrap(), "Foo artist"); // Sanity check
	assert_eq!(ilst.genre().unwrap(), "Funk");
}

#[test_log::test]
fn gnre_conversion_case_2() {
	// Case 2: 1 `gnre` atom present, 1 `©gen` present. `gnre` gets discarded.
	let ilst = read_ilst_bestattempt("tests/tags/assets/ilst/gnre_conversion_case_2.ilst");

	assert_eq!(ilst.len(), 2);
	assert_eq!(ilst.artist().unwrap(), "Foo artist"); // Sanity check
	assert_eq!(ilst.genre().unwrap(), "My Custom Genre");
}

#[test_log::test]
fn gnre_conversion_case_3() {
	// Case 2: 1 `gnre` atom present, 1 `©gen` present. implicit conversion are disabled, `gnre` is retained
	//         as an unknown atom.
	let ilst = read_ilst(
		"tests/tags/assets/ilst/gnre_conversion_case_2.ilst",
		ParseOptions::new().implicit_conversions(false),
	);

	assert_eq!(ilst.len(), 3);
	assert_eq!(ilst.artist().unwrap(), "Foo artist"); // Sanity check
	assert_eq!(ilst.genre().unwrap(), "My Custom Genre");
	assert_eq!(
		ilst.get(&AtomIdent::Fourcc(*b"gnre"))
			.unwrap()
			.data()
			.next()
			.unwrap(),
		&AtomData::Unknown {
			code: DataType::BeSignedInteger,
			data: vec![0, 6]
		}
	);
}

#[test_log::test]
fn retain_known_idents_with_unknown_types() {
	// When we convert an `Atom` -> `TagItem`, we push each value as its own item. Since atoms can
	// have multiple values of different types, we need to make sure we retain any values we can't
	// convert.
	let mut ilst = Ilst::new();

	let mut atom = Atom::new(
		AtomIdent::Freeform {
			mean: Cow::Borrowed("com.apple.iTunes"),
			name: Cow::Borrowed("ARTISTS"),
		},
		AtomData::UTF8(String::from("Serial-ATA")),
	);

	atom.push_data(AtomData::UTF8(String::from("Lofty")));
	atom.push_data(AtomData::UnsignedInteger(42)); // Something unexpected

	ilst.insert(atom);

	let (remainder, tag) = ilst.split_tag();

	assert_eq!(remainder.len(), 1);
	let mut strings = tag.get_strings(ItemKey::TrackArtists);
	assert_eq!(strings.next().unwrap(), "Serial-ATA");
	assert_eq!(strings.next().unwrap(), "Lofty");
}

#[test_log::test]
fn picture_roundtrip() {
	let mut ilst = Ilst::new();
	let picture = Picture::unchecked(vec![1, 2, 3]).build();
	let picture2 = Picture::unchecked(vec![4, 5, 6]).build();

	ilst.insert_picture(picture.clone());
	ilst.insert_picture(picture2.clone());
	assert_eq!(ilst.pictures().unwrap().count(), 2);

	let tag: Tag = ilst.into();
	assert_eq!(tag.pictures().len(), 2);

	let ilst: Ilst = tag.into();
	assert_eq!(ilst.pictures().unwrap().count(), 2);
}
