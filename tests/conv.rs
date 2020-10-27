use audiotags::{Tag, TagType};

#[test]
fn test_convert_mp3_to_mp4() {
    // we have an mp3 and an m4a file
    const MP3_FILE: &'static str = "assets/a.mp3";
    const M4A_FILE: &'static str = "assets/a.m4a";
    // read tag from the mp3 file. Using `default()` so that the type of tag is guessed from the file extension
    let mut mp3tag = Tag::default().read_from_path(MP3_FILE).unwrap();
    // set the title
    mp3tag.set_title("title from mp3 file");
    // we can convert it to an mp4 tag and save it to an m4a file.
    let mut mp4tag = mp3tag.into_tag(TagType::Mp4);
    mp4tag.write_to_path(M4A_FILE).unwrap();

    // reload the tag from the m4a file; this time specifying the tag type (you can also use `default()`)
    let mp4tag_reload = Tag::with_tag_type(TagType::Mp4)
        .read_from_path(M4A_FILE)
        .unwrap();
    // the tag originated from an mp3 file is successfully written to an m4a file!
    assert_eq!(mp4tag_reload.title(), Some("title from mp3 file"));
}
