use crate::{APE, ID3V1, ID3V2, ILST, RIFF_INFO, VORBIS_COMMENTS};

use lofty::ape::ApeTag;
use lofty::id3::v1::Id3v1Tag;
use lofty::id3::v2::{FrameValue, Id3v2Tag, LanguageFrame, TextEncoding};
use lofty::iff::RiffInfoList;
use lofty::mp4::{AtomData, AtomIdent, Ilst};
use lofty::ogg::VorbisComments;
use lofty::{ItemKey, ItemValue, Tag, TagType};

fn create_tag(tag_type: TagType) -> Tag {
	let mut tag = Tag::new(tag_type);

	tag.insert_text(ItemKey::TrackTitle, String::from("Foo title"));
	tag.insert_text(ItemKey::TrackArtist, String::from("Bar artist"));
	tag.insert_text(ItemKey::AlbumTitle, String::from("Baz album"));
	tag.insert_text(ItemKey::Comment, String::from("Qux comment"));
	tag.insert_text(ItemKey::TrackNumber, String::from("1"));
	tag.insert_text(ItemKey::Genre, String::from("Classical"));

	tag
}

fn verify_tag(tag: &Tag, track_number: bool, genre: bool) {
	assert_eq!(tag.get_string(&ItemKey::TrackTitle), Some("Foo title"));
	assert_eq!(tag.get_string(&ItemKey::TrackArtist), Some("Bar artist"));
	assert_eq!(tag.get_string(&ItemKey::AlbumTitle), Some("Baz album"));
	assert_eq!(tag.get_string(&ItemKey::Comment), Some("Qux comment"));

	if track_number {
		assert_eq!(tag.get_string(&ItemKey::TrackNumber), Some("1"));
	}

	if genre {
		assert_eq!(tag.get_string(&ItemKey::Genre), Some("Classical"));
	}
}

#[test]
fn ape_to_tag() {
	let ape = ApeTag::read_from(&mut std::io::Cursor::new(&APE[..])).unwrap();

	let tag: Tag = ape.into();

	verify_tag(&tag, true, true);
}

#[test]
fn tag_to_ape() {
	fn verify_key(tag: &ApeTag, key: &str, expected_val: &str) {
		assert_eq!(
			tag.get_key(key).map(|i| i.value()),
			Some(&ItemValue::Text(String::from(expected_val)))
		);
	}

	let tag = create_tag(TagType::Ape);

	let ape_tag: ApeTag = tag.into();

	verify_key(&ape_tag, "Title", "Foo title");
	verify_key(&ape_tag, "Artist", "Bar artist");
	verify_key(&ape_tag, "Album", "Baz album");
	verify_key(&ape_tag, "Comment", "Qux comment");
	verify_key(&ape_tag, "Track", "1");
	verify_key(&ape_tag, "Genre", "Classical");
}

#[test]
fn id3v1_to_tag() {
	let id3v1 = Id3v1Tag::read_from(ID3V1);

	let tag: Tag = id3v1.into();

	verify_tag(&tag, true, true);
}

#[test]
fn tag_to_id3v1() {
	let tag = create_tag(TagType::Id3v1);

	let id3v1_tag: Id3v1Tag = tag.into();

	assert_eq!(id3v1_tag.title.as_deref(), Some("Foo title"));
	assert_eq!(id3v1_tag.artist.as_deref(), Some("Bar artist"));
	assert_eq!(id3v1_tag.album.as_deref(), Some("Baz album"));
	assert_eq!(id3v1_tag.comment.as_deref(), Some("Qux comment"));
	assert_eq!(id3v1_tag.track_number, Some(1));
	assert_eq!(id3v1_tag.genre, Some(32));
}

#[test]
fn id3v2_to_tag() {
	let id3v2 = Id3v2Tag::read_from(&mut &ID3V2[..]).unwrap();

	let tag: Tag = id3v2.into();

	verify_tag(&tag, true, true);
}

#[test]
fn tag_to_id3v2() {
	fn verify_frame(tag: &Id3v2Tag, id: &str, value: &str) {
		let frame = tag.get(id);

		assert!(frame.is_some());

		let frame = frame.unwrap();

		assert_eq!(
			frame.content(),
			&FrameValue::Text {
				encoding: TextEncoding::UTF8,
				value: String::from(value)
			}
		);
	}

	let tag = create_tag(TagType::Id3v2);

	let id3v2_tag: Id3v2Tag = tag.into();

	verify_frame(&id3v2_tag, "TIT2", "Foo title");
	verify_frame(&id3v2_tag, "TPE1", "Bar artist");
	verify_frame(&id3v2_tag, "TALB", "Baz album");

	let frame = id3v2_tag.get("COMM").unwrap();
	assert_eq!(
		frame.content(),
		&FrameValue::Comment(LanguageFrame {
			encoding: TextEncoding::Latin1,
			language: String::from("eng"),
			description: String::new(),
			content: String::from("Qux comment")
		})
	);

	verify_frame(&id3v2_tag, "TRCK", "1");
	verify_frame(&id3v2_tag, "TCON", "Classical");
}

#[test]
fn ilst_to_tag() {
	let ilst = Ilst::read_from(&mut &ILST[..], (ILST.len() - 1) as u64).unwrap();

	let tag: Tag = ilst.into();

	verify_tag(&tag, false, true);
}

#[test]
fn tag_to_ilst() {
	fn verify_atom(ilst: &Ilst, ident: [u8; 4], data: &str) {
		let atom = ilst.atom(&AtomIdent::Fourcc(ident)).unwrap();

		let data = AtomData::UTF8(String::from(data));

		assert_eq!(atom.data(), &data);
	}

	let tag = create_tag(TagType::Mp4Atom);

	let ilst: Ilst = tag.into();

	verify_atom(&ilst, *b"\xa9nam", "Foo title");
	verify_atom(&ilst, *b"\xa9ART", "Bar artist");
	verify_atom(&ilst, *b"\xa9alb", "Baz album");
	verify_atom(&ilst, *b"\xa9cmt", "Qux comment");
	verify_atom(&ilst, *b"\xa9gen", "Classical");
}

#[test]
fn riff_info_to_tag() {
	let riff_info = RiffInfoList::read_from(
		&mut std::io::Cursor::new(&RIFF_INFO),
		(RIFF_INFO.len() - 1) as u64,
	)
	.unwrap();

	let tag: Tag = riff_info.into();

	verify_tag(&tag, true, false);
}

#[test]
fn tag_to_riff_info() {
	let tag = create_tag(TagType::RiffInfo);

	let riff_info: RiffInfoList = tag.into();

	assert_eq!(riff_info.get("INAM"), Some("Foo title"));
	assert_eq!(riff_info.get("IART"), Some("Bar artist"));
	assert_eq!(riff_info.get("IPRD"), Some("Baz album"));
	assert_eq!(riff_info.get("ICMT"), Some("Qux comment"));
	assert_eq!(riff_info.get("IPRT"), Some("1"));
}

#[test]
fn vorbis_comments_to_tag() {
	let vorbis_comments = VorbisComments::read_from(&mut &VORBIS_COMMENTS[..]).unwrap();

	let tag: Tag = vorbis_comments.into();

	verify_tag(&tag, true, true);
}

#[test]
fn tag_to_vorbis_comments() {
	let tag = create_tag(TagType::VorbisComments);

	let vorbis_comments: VorbisComments = tag.into();

	assert_eq!(vorbis_comments.get_item("TITLE"), Some("Foo title"));
	assert_eq!(vorbis_comments.get_item("ARTIST"), Some("Bar artist"));
	assert_eq!(vorbis_comments.get_item("ALBUM"), Some("Baz album"));
	assert_eq!(vorbis_comments.get_item("COMMENT"), Some("Qux comment"));
	assert_eq!(vorbis_comments.get_item("TRACKNUMBER"), Some("1"));
	assert_eq!(vorbis_comments.get_item("GENRE"), Some("Classical"));
}
