<img align="right" width="200" height="200" src="doc/lofty.svg" alt="Lofty logo">

# Lofty

*Parse, convert, and write metadata to various audio formats.*

[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/Serial-ATA/lofty-rs/ci.yml?branch=main&logo=github&style=for-the-badge)](https://github.com/Serial-ATA/lofty-rs/actions/workflows/ci.yml)
[![Downloads](https://img.shields.io/crates/d/lofty?style=for-the-badge&logo=rust)](https://crates.io/crates/lofty)
[![Version](https://img.shields.io/crates/v/lofty?style=for-the-badge&logo=rust)](https://crates.io/crates/lofty)
[![Documentation](https://img.shields.io/badge/docs.rs-lofty-informational?style=for-the-badge&logo=read-the-docs)](https://docs.rs/lofty/)

⚠️ **LOOKING FOR HELP WITH DOCUMENTATION** ⚠️

I'm looking for help with the refinement of the docs. Any contribution, whether it be typos,
grammar, punctuation, or missing examples is highly appreciated!

## Supported Formats

[See here](./SUPPORTED_FORMATS.md)

## Examples

* [Tag reader](examples/tag_reader.rs)
* [Tag stripper](examples/tag_stripper.rs)
* [Tag writer](examples/tag_writer.rs)
* [Custom resolver](examples/custom_resolver)

To try them out, run:

```bash
cargo run --example tag_reader /path/to/file
cargo run --example tag_stripper /path/to/file
cargo run --example tag_writer <options> /path/to/file
cargo run --example custom_resolver
```

## Documentation

Available [here](https://docs.rs/lofty)

## Benchmarking

There are benchmarks available [here](./benches).

These benchmarks make use of [criterion](https://github.com/bheisler/criterion.rs), and
due to its size it cannot be a normal dev-dependency. To run the benchmarks do:

```shell
RUSTFLAGS="--cfg bench" cargo bench
```

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
