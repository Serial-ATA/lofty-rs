# Lofty
[![GitHub Workflow Status](https://img.shields.io/github/workflow/status/Serial-ATA/lofty-rs/CI?style=for-the-badge&logo=github)](https://github.com/Serial-ATA/lofty-rs/actions/workflows/ci.yml)
[![Downloads](https://img.shields.io/crates/d/lofty?style=for-the-badge&logo=rust)](https://crates.io/crates/lofty)
[![Version](https://img.shields.io/crates/v/lofty?style=for-the-badge&logo=rust)](https://crates.io/crates/lofty)
[![Documentation](https://img.shields.io/badge/docs.rs-lofty-informational?style=for-the-badge&logo=read-the-docs)](https://docs.rs/lofty/)

Parse, convert, and write metadata to various audio formats.

## Supported Formats

| File Format | Metadata Format(s)                   |
|-------------|--------------------------------------|
| Ape         | `APEv2`, `APEv1`, `ID3v2`\*, `ID3v1` |
| AIFF        | `ID3v2`, `Text Chunks`               |
| FLAC        | `Vorbis Comments`, `ID3v2`\*         |
| MP3         | `ID3v2`, `ID3v1`, `APEv2`, `APEv1`   |
| MP4         | `iTunes-style ilst`                  |
| Opus        | `Vorbis Comments`                    |
| Ogg Vorbis  | `Vorbis Comments`                    |
| Speex       | `Vorbis Comments`                    |
| WAV         | `ID3v2`, `RIFF INFO`                 |
| WavPack     | `APEv2`, `APEv1`, `ID3v1`            |

\* The tag will be **read only**, due to lack of official support

## Examples

* [Tag reader](examples/tag_reader.rs)
* [Tag stripper](examples/tag_stripper.rs)
* [Tag writer](examples/tag_writer.rs)

To try them out, run:

```bash
cargo run --example tag_reader /path/to/file
cargo run --example tag_stripper /path/to/file
cargo run --example tag_writer <options> /path/to/file
```

## Documentation

Available [here](https://docs.rs/lofty)

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
