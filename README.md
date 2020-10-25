# audiotags

This crate makes it easier to parse tags/metadata in audio files of different file types.

This crate aims to provide a unified trait for parsers and writers of different audio file formats. This means that you can parse tags in mp3 and m4a files with a single function: `audiotags::from_path()` and get fields by directly calling `.album()`, `.artist()` on its result. Without this crate, you would otherwise need to learn different APIs in **id3**, **mp4ameta** crates in order to parse metadata in different file foramts.

## Example

```rust
use audiotags;

fn main() {
    const MP3: &'static str = "a.mp3";
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

    const M4A: &'static str = "b.m4a";
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