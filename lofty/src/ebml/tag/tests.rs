use crate::ebml::MatroskaTag;
use crate::prelude::ItemKey;
use crate::tag::{Accessor, Tag, TagType};

#[test_log::test]
fn tag_to_matroska_tag() {
	let mut tag = Tag::new(TagType::Matroska);

	tag.insert_text(ItemKey::TrackArtist, String::from("Foo artist"));
	tag.insert_text(ItemKey::TrackTitle, String::from("Bar title"));
	tag.insert_text(ItemKey::AlbumTitle, String::from("Baz album"));
	tag.insert_text(ItemKey::TrackNumber, String::from("1"));
	tag.insert_text(ItemKey::TrackTotal, String::from("2"));

	let matroska_tag: MatroskaTag = tag.into();

	assert_eq!(matroska_tag.artist().as_deref(), Some("Foo artist"));
	assert_eq!(matroska_tag.title().as_deref(), Some("Bar title"));
	assert_eq!(matroska_tag.album().as_deref(), Some("Baz album"));
	assert_eq!(matroska_tag.track(), Some(1));
	assert_eq!(matroska_tag.track_total(), Some(2));
}
