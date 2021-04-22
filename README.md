# Lofty
[![Crate](https://img.shields.io/crates/v/lofty.svg)](https://crates.io/crates/lofty)
[![Crate](https://img.shields.io/crates/d/lofty.svg)](https://crates.io/crates/lofty)
[![Crate](https://img.shields.io/crates/l/lofty.svg)](https://crates.io/crates/lofty)
[![Documentation](https://docs.rs/lofty/badge.svg)](https://docs.rs/lofty/)

This is a fork of [Audiotags](https://github.com/TianyiShi2001/audiotags), adding support for more file types and (optionally) duration.

Parse, convert, and write metadata to audio files of different file types.

**Lofty** aims to provide a unified trait for parsers and writers of different audio file formats.
Without this crate, you would otherwise need to learn the different APIs in **id3**, **mp4ameta**, etc.
in order to parse metadata in different file formats.

## Supported Formats

| File Format | Extensions                                | Read | Write | Metadata Format(s)   |
|-------------|-------------------------------------------|------|-------|----------------------|
| Ape         | `ape`                                     |**X** |**X**  | `APEv2`              |
| AIFF        | `aiff`, `aif`                             |**X** |**X**  | `ID3v2`              |
| FLAC        | `flac`                                    |**X** |**X**  | `Vorbis Comments`    |
| MP3         | `mp3`                                     |**X** |**X**  | `ID3v2`              |
| MP4         | `mp4`, `m4a`, `m4b`, `m4p`, `m4v`, `isom` |**X** |**X**  | `Vorbis Comments`    |
| Opus        | `opus`                                    |**X** |       | `Vorbis Comments`    |
| Ogg         | `ogg`, `oga`                              |**X** |**X**  | `Vorbis Comments`    |
| WAV         | `wav`, `wave`                             |**X** |**X**  | `RIFF INFO`, `ID3v2` |

## Documentation

Available [here](https://docs.rs/lofty)

## Thanks

All these great projects helped make this crate possible. (*Sorted alphabetically*)

* [**ape**](https://github.com/rossnomann/rust-ape)
* [**id3**](https://github.com/polyfloyd/rust-id3)
* [**lewton**](https://github.com/RustAudio/lewton)
* [**metaflac**](https://github.com/jameshurst/rust-metaflac)
* [**mp4ameta**](https://github.com/Saecki/rust-mp4ameta)
* [**ogg**](https://github.com/RustAudio/ogg)
* [**opus_headers**](https://github.com/zaethan/opus_headers)
* [**riff**](https://github.com/frabert/riff)

## License

Licensed under either of

* Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license
  ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
