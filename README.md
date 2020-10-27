# audiotags

[![Crate](https://img.shields.io/crates/v/audiotags.svg)](https://crates.io/crates/audiotags)
[![Crate](https://img.shields.io/crates/d/audiotags.svg)](https://crates.io/crates/audiotags)
[![Crate](https://img.shields.io/crates/l/audiotags.svg)](https://crates.io/crates/audiotags)
[![Documentation](https://docs.rs/audiotags/badge.svg)](https://docs.rs/audiotags/)
[![Manual](https://img.shields.io/badge/RTFM-Manual-blue)](https://tianyishi2001.github.io/audiotags)

**audiotags** makes it easier to **parse, convert and write metadata** (a.k.a tag) in audio files of different file types.

This crate aims to provide a unified trait for parsers and writers of different audio file formats. This means that you can parse tags in mp3, flac, and m4a files with a single function: `Tag::default().read_from_path()` and get fields by directly calling `.album()`, `.artist()` on its result. Without this crate, you would otherwise need to learn different APIs in **id3**, **mp4ameta** etc. in order to parse metadata in different file formats.

## Examples

Examples can be found in the [manual](https://tianyishi2001.github.io/audiotags).

## Performance

Using **audiotags** incurs a little overhead due to vtables if you want to guess the metadata format (from file extension). Apart from this the performance is almost the same as directly calling function provided by those 'specialized' crates. (It is possible to use **audiotags** *without* dynamic dispatch, in which case you need to specify the tag type but benefit from speed improvement).

No copies will be made if you only need to read and write metadata of one format. If you want to convert between tags, copying is unavoidable no matter if you use **audiotags** or use getters and setters provided by specialized libraries. **audiotags** is not making additional unnecessary copies.

Theoretically it is possible to achieve zero-copy conversions if all parsers can parse into a unified struct. However, this is going to be a lot of work. I might be able to implement them, but it will be no sooner than the Christmas vacation.

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
}
```