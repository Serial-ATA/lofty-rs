use audiotags::{AnyTag, AudioTagEdit, Id3v2Tag};

#[test]
fn test_anytag() {
    let mut tag = AnyTag::default();
    tag.set_title("foo");
    tag.set_year(2001);
    let tag: Id3v2Tag = tag.into();
    assert_eq!(tag.year(), Some(2001));
}
