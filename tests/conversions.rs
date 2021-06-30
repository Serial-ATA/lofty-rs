use lofty::{Id3Format, OggFormat, Tag, TagType};

macro_rules! convert_tag {
	($tag: ident) => {
		assert_eq!($tag.title(), Some("Title Updated"));
		assert_eq!($tag.artist_str(), Some("Artist Updated"));
		assert_eq!($tag.album_artist_str(), Some("Album Artist Updated"));
	};
}

#[test]
fn test_conversions() {
	let mut tag = Tag::new().read_from_path("tests/assets/a.mp3").unwrap();

	tag.set_title("Title Updated");
	tag.set_artist("Artist Updated");
	tag.set_album_artist("Album Artist Updated");
	convert_tag!(tag);

	let tag = tag.to_dyn_tag(TagType::Ape);
	convert_tag!(tag);

	let tag = tag.to_dyn_tag(TagType::Mp4);
	convert_tag!(tag);

	let tag = tag.to_dyn_tag(TagType::RiffInfo);
	convert_tag!(tag);

	let tag = tag.to_dyn_tag(TagType::Ogg(OggFormat::Vorbis));
	convert_tag!(tag);

	let tag = tag.to_dyn_tag(TagType::Id3v2(Id3Format::Form));
	convert_tag!(tag);
}
