# Lofty
[![GitHub Workflow Status](https://img.shields.io/github/workflow/status/Serial-ATA/lofty-rs/CI?style=for-the-badge&logo=github)](https://github.com/Serial-ATA/lofty-rs/actions/workflows/ci.yml)
[![Downloads](https://img.shields.io/crates/d/lofty?style=for-the-badge&logo=rust)](https://crates.io/crates/lofty)
[![Version](https://img.shields.io/crates/v/lofty?style=for-the-badge&logo=rust)](https://crates.io/crates/lofty)
[![Documentation](https://img.shields.io/badge/docs.rs-lofty-informational?style=for-the-badge&logo=read-the-docs)](https://docs.rs/lofty/)

This is a fork of [Audiotags](https://github.com/TianyiShi2001/audiotags), adding support for more file types.

Parse, convert, and write metadata to various audio formats.

## Supported Formats

| File Format | Extensions                                | Read | Write | Metadata Format(s)  |
|-------------|-------------------------------------------|------|-------|---------------------|
| Ape         | `ape`                                     |**X** |**X**  |`APEv2`              |
| AIFF        | `aiff`, `aif`                             |**X** |**X**  |`ID3v2`              |
| FLAC        | `flac`                                    |**X** |**X**  |`Vorbis Comments`    |
| MP3         | `mp3`                                     |**X** |**X**  |`ID3v2`              |
| MP4         | `mp4`, `m4a`, `m4b`, `m4p`, `m4v`, `isom` |**X** |**X**  |`Vorbis Comments`    |
| Opus        | `opus`                                    |**X** |**X**  |`Vorbis Comments`    |
| Ogg         | `ogg`, `oga`                              |**X** |**X**  |`Vorbis Comments`    |
| WAV         | `wav`, `wave`                             |**X** |**X**  |`RIFF INFO`, `ID3v2` |

## Documentation

Available [here](https://docs.rs/lofty)

## Thanks

All these great projects helped make this crate possible. (*Sorted alphabetically*)

* [**ape**](https://github.com/rossnomann/rust-ape)
* [**id3**](https://github.com/polyfloyd/rust-id3)
* [**metaflac**](https://github.com/jameshurst/rust-metaflac)
* [**mp4ameta**](https://github.com/Saecki/rust-mp4ameta)
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
