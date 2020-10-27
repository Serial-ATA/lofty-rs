# audiotags

[![Crate](https://img.shields.io/crates/v/audiotags.svg)](https://crates.io/crates/audiotags)
[![Crate](https://img.shields.io/crates/d/audiotags.svg)](https://crates.io/crates/audiotags)
[![Crate](https://img.shields.io/crates/l/audiotags.svg)](https://crates.io/crates/audiotags)
[![Documentation](https://docs.rs/audiotags/badge.svg)](https://docs.rs/audiotags/)
[![Manual](https://img.shields.io/badge/RTFM-Manual-blue)](https://tianyishi2001.github.io/audiotags)

This crate makes it easier to parse, convert and write metadata (a.k.a tag) in audio files of different file types.

This crate aims to provide a unified trait for parsers and writers of different audio file formats. This means that you can parse tags in mp3, flac, and m4a files with a single function: `Tag::default().read_from_path()` and get fields by directly calling `.album()`, `.artist()` on its result. Without this crate, you would otherwise need to learn different APIs in **id3**, **mp4ameta** etc. in order to parse metadata in different file formats.

## Performance

Using **audiotags** incurs a little overhead due to vtables if you want to guess the metadata format (from file extension). Apart from this the performance is almost the same as directly calling function provided by those 'specialized' crates.

No copies will be made if you only need to read and write metadata of one format. If you want to convert between tags, copying is unavoidable no matter if you use **audiotags** or use getters and setters provided by specialized libraries. **audiotags** is not making additional unnecessary copies.

Theoretically it is possible to achieve zero-copy conversions if all parsers can parse into a unified struct. However, this is going to be a lot of work. I might be able to implement them, but it will be no sooner than the Christmas vacation.

## Example

(Due to copyright restrictions I cannot upload the actual audio files here)

```rust
use audiotags::Tag;
fn main() {
    const MP3: &'static str = "a.mp3";
    let mut tags = Tag::default().read_from_path(MP3).unwrap();
    // without this crate you would call id3::Tag::read_from_path()
    println!("Title: {:?}", tags.title());
    println!("Artist: {:?}", tags.artist());
    tags.set_album_artist("CINDERELLA PROJECT");
    let album = tags.album().unwrap();
    println!("Album title and artist: {:?}", (album.title, album.artist));
    println!("Track: {:?}", tags.track());
    tags.write_to_path(MP3).unwrap();
    // Title: Some("お願い！シンデレラ")
    // Artist: Some("高垣楓、城ヶ崎美嘉、小日向美穂、十時愛梨、川島瑞樹、日野茜、輿水幸子、佐久間まゆ、白坂小梅")
    // Album title and artist: ("THE IDOLM@STER CINDERELLA GIRLS ANIMATION PROJECT 01 Star!!", Some("CINDERELLA PROJECT"))
    // Track: (Some(2), Some(4))
    const M4A: &'static str = "b.m4a";
    let mut tags = Tag::default().read_from_path(M4A).unwrap();
    // without this crate you would call mp4ameta::Tag::read_from_path()
    println!("Title: {:?}", tags.title());
    println!("Artist: {:?}", tags.artist());
    let album = tags.album().unwrap();
    println!("Album title and artist: {:?}", (album.title, album.artist));
    tags.set_total_tracks(4);
    println!("Track: {:?}", tags.track());
    tags.write_to_path(M4A).unwrap();
    // Title: Some("ふわふわ時間")
    // Artist: Some("桜高軽音部 [平沢唯・秋山澪・田井中律・琴吹紬(CV:豊崎愛生、日笠陽子、佐藤聡美、寿美菜子)]")
    // Album title and artist: ("ふわふわ時間", Some("桜高軽音部 [平沢唯・秋山澪・田井中律・琴吹紬(CV:豊崎愛生、日笠陽子、佐藤聡美、寿美菜子)]"))
    // Track: (Some(1), Some(4))
    const FLAC: &'static str = "c.flac";
    let mut tags = Tag::default().read_from_path(FLAC).unwrap();
    // without this crate you would call metaflac::Tag::read_from_path()
    println!("Title: {:?}", tags.title());
    println!("Artist: {:?}", tags.artist());
    let album = tags.album().unwrap();
    println!("Album title and artist: {:?}", (album.title, album.artist));
    tags.set_year(2017);
    println!("Year: {:?}", tags.year());
    tags.write_to_path(FLAC).unwrap();
    // Title: Some("意味/無/ジュニーク・ニコール")
    // Artist: Some("岡部啓一")
    // Album title and artist: ("NieR:Automata Original Soundtrack", Some("SQUARE ENIX"))
    // Year: Some(2017)
}
```

You can convert between different tag types:

```rust
use audiotags::{Tag, TagType};

fn main() {
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
    mp4tag.set_config(Config::default().sep_artist("/"));
    // separator is by default `;` but we can customise it
    let mp3tag = mp4tag.into_tag(TagType::Id3v2);
    assert_eq!(mp3tag.artist(), Some("artist1 of mp4/artist2 of mp4"));
}
```

## Supported Formats

| File Fomat    | Metadata Format       | backend                                                     |
| ------------- | --------------------- | ----------------------------------------------------------- |
| `mp3`         | id3v2.4               | [**id3**](https://github.com/polyfloyd/rust-id3)            |
| `m4a/mp4/...` | MPEG-4 audio metadata | [**mp4ameta**](https://github.com/Saecki/rust-mp4ameta)     |
| `flac`        | Vorbis comment        | [**metaflac**](https://github.com/jameshurst/rust-metaflac) |

## Getters and Setters

```rust
pub trait AudioTagEdit{
    fn title(&self) -> Option<&str>;
    fn set_title(&mut self, title: &str);
    fn remove_title(&mut self);
    fn artist(&self) -> Option<&str>;
    fn remove_artist(&mut self);
    fn set_artist(&mut self, artist: &str);
    fn year(&self) -> Option<i32>;
    fn set_year(&mut self, year: i32);
    fn remove_year(&mut self);
    fn album(&self) -> Option<Album>;
    fn remove_album(&mut self);
    fn album_title(&self) -> Option<&str>;
    fn remove_album_title(&mut self);
    fn album_artist(&self) -> Option<&str>;
    fn remove_album_artist(&mut self);
    fn album_cover(&self) -> Option<Picture>;
    fn remove_album_cover(&mut self);
    fn set_album(&mut self, album: Album);
    fn set_album_title(&mut self, v: &str);
    fn set_album_artist(&mut self, v: &str);
    fn set_album_cover(&mut self, cover: Picture);
    fn track(&self) -> (Option<u16>, Option<u16>);
    fn set_track(&mut self, track: (u16, u16));
    fn remove_track(&mut self);
    fn track_number(&self) -> Option<u16>;
    fn set_track_number(&mut self, track_number: u16);
    fn remove_track_number(&mut self);
    fn total_tracks(&self) -> Option<u16>;
    fn set_total_tracks(&mut self, total_track: u16);
    fn remove_total_tracks(&mut self);
    fn disc(&self) -> (Option<u16>, Option<u16>);
    fn set_disc(&mut self, disc: (u16, u16));
    fn remove_disc(&mut self);
    fn disc_number(&self) -> Option<u16>;
    fn set_disc_number(&mut self, disc_number: u16);
    fn remove_disc_number(&mut self);
    fn total_discs(&self) -> Option<u16>;
    fn set_total_discs(&mut self, total_discs: u16);
    fn remove_total_discs(&mut self);
    fn write_to(&mut self, file: &mut File) -> Result<(), BoxedError>;
    // cannot use impl AsRef<Path>
    fn write_to_path(&mut self, path: &str) -> Result<(), BoxedError>;
}
```