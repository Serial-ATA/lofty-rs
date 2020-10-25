# audiotags

[![Crate](https://img.shields.io/crates/v/audiotags.svg)](https://crates.io/crates/audiotags)
[![Crate](https://img.shields.io/crates/d/audiotags.svg)](https://crates.io/crates/audiotags)
[![Crate](https://img.shields.io/crates/l/audiotags.svg)](https://crates.io/crates/audiotags)
[![Documentation](https://docs.rs/audiotags/badge.svg)](https://docs.rs/audiotags/)

This crate makes it easier to parse tags/metadata in audio files of different file types.

This crate aims to provide a unified trait for parsers and writers of different audio file formats. This means that you can parse tags in mp3 and m4a files with a single function: `audiotags::from_path()` and get fields by directly calling `.album()`, `.artist()` on its result. Without this crate, you would otherwise need to learn different APIs in **id3**, **mp4ameta** crates in order to parse metadata in different file foramts.

## Example

(Due to copyright restrictions I cannot upload the actual audio files here)

```rust
use audiotags;

fn main() {
    const MP3: &'static str = "お願い！シンデレラ.mp3";
    let mut tags = audiotags::from_path(MP3).unwrap();
    // without this crate you would call id3::Tag::from_path()
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

    const M4A: &'static str = "ふわふわ時間.m4a";
    let mut tags = audiotags::from_path(M4A).unwrap();
    // without this crate you would call mp4ameta::Tag::from_path()
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
}
```

## Supported Methods

```rust
pub trait AudioTagsIo {
    fn title(&self) -> Option<&str>;
    fn set_title(&mut self, title: &str);
    fn artist(&self) -> Option<&str>;
    fn set_artist(&mut self, artist: &str);
    fn year(&self) -> Option<i32>;
    fn set_year(&mut self, year: i32);
    fn album(&self) -> Option<Album>;
    fn album_title(&self) -> Option<&str>;
    fn album_artist(&self) -> Option<&str>;
    fn album_cover(&self) -> Option<Picture>;
    fn set_album(&mut self, album: Album);
    fn set_album_title(&mut self, v: &str);
    fn set_album_artist(&mut self, v: &str);
    fn set_album_cover(&mut self, cover: Picture);
    fn track(&self) -> (Option<u16>, Option<u16>);
    fn set_track_number(&mut self, track_number: u16);
    fn set_total_tracks(&mut self, total_track: u16);
    fn disc(&self) -> (Option<u16>, Option<u16>);
    fn set_disc_number(&mut self, disc_number: u16);
    fn set_total_discs(&mut self, total_discs: u16);
    fn write_to(&self, file: &File) -> Result<(), BoxedError>;
    // cannot use impl AsRef<Path>
    fn write_to_path(&self, path: &str) -> Result<(), BoxedError>;
}
```