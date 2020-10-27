use audiotags::{Config, Tag, TagType};

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
    let mut mp4tag = Tag::with_tag_type(TagType::Mp4)
        .read_from_path(M4A_FILE)
        .unwrap();
    // the tag originated from an mp3 file is successfully written to an m4a file!
    assert_eq!(mp4tag.title(), Some("title from mp3 file"));
    // multiple artists
    mp4tag.add_artist("artist1 of mp4");
    mp4tag.add_artist("artist2 of mp4");
    assert_eq!(
        mp4tag.artists(),
        Some(vec!["artist1 of mp4", "artist2 of mp4"])
    );
    // convert to id3 tag, which does not support multiple artists
    let mp3tag = mp4tag
        .with_config(Config::default().sep_artist("/")) // separator is by default `;`
        .into_tag(TagType::Id3v2);
    assert_eq!(mp3tag.artist(), Some("artist1 of mp4/artist2 of mp4"));
}
