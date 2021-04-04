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

# Supported Formats

| File Format   | Metadata Format | Backend                                                     |
|---------------|-----------------|-------------------------------------------------------------|
| `mp3`         | ID3v2.4         | [**id3**](https://github.com/polyfloyd/rust-id3)            |
| `wav`         | TODO            | TODO                                                        |
| `ape`         | TODO            | TODO                                                        |
| `opus`        | Vorbis Comment  | [**opus_headers**](https://github.com/zaethan/opus_headers) |
| `ogg`         | Vorbis Comment  | [**lewton**](https://github.com/RustAudio/lewton)           |
| `flac`        | Vorbis Comment  | [**metaflac**](https://github.com/jameshurst/rust-metaflac) |
| `m4a/mp4/...` | Vorbis Comment  | [**mp4ameta**](https://github.com/Saecki/rust-mp4ameta)     |

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
