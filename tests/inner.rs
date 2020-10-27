use audiotags::*;

#[test]
fn test_inner() {
    let mut innertag = metaflac::Tag::default();
    innertag
        .vorbis_comments_mut()
        .set_title(vec!["title from metaflac::Tag"]);
    let tag: FlacTag = innertag.into();
    let mut id3tag = tag.into_tag(TagType::Id3v2);
    id3tag.write_to_path("assets/a.mp3").unwrap();

    let id3tag_reload = Tag::default().read_from_path("assets/a.mp3").unwrap();
    assert_eq!(id3tag_reload.title(), Some("title from metaflac::Tag"));
}
