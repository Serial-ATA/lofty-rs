# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- Removed a bad assertion when writing nil packets ([PR](https://github.com/Serial-ATA/lofty-rs/pull/547))

## [0.7.0] - 2025-01-05

### Fixed
- Writing a packet whose size is perfectly divisible by 255 would make the second to last segment have a size of 0, rather than 255 ([issue](https://github.com/Serial-ATA/lofty-rs/issues/469)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/475))

### Removed
- `Page::extend()` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/475))
- `segment_table()` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/475))

## [0.6.1] - 2024-04-21

### Fixed
- When writing large packets, the size would slowly shift out of sync, causing the pages to be written incorrectly. ([issue](https://github.com/Serial-ATA/lofty-rs/issues/350)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/375))

## [0.6.0] - 2024-01-03

### Added
- `Packets::iter()`

## [0.5.0] - 2023-01-29

### Added
- `Packets::{len, is_empty}`

### Changed
- `Packets::write_to` will now return the number of pages written
- Segment tables are now stored in `PageHeader`
- Limit maximum written page size to ~8KB

### Fixed
- `Packets::read_count` will properly validate that the correct number of packets were read

## [0.4.0] - 2022-12-27

### Added
- Support for reading packets with the new `Packets` struct. ([PR](https://github.com/Serial-ATA/lofty-rs/pull/76))
- `PageHeader` struct. ([PR](https://github.com/Serial-ATA/lofty-rs/pull/76))

### Changed
- The reading of OGG files has switched to using packets opposed to pages, making it more spec-compliant and efficient.
- Most fields in `Page` have been separated out into the new `PageHeader` struct.
- `paginate` now works with a collection of packets. ([PR](https://github.com/Serial-ATA/lofty-rs/pull/79))

### Removed
- Removed `Page::new`, now pages can only be created through `ogg_pager::paginate` or
  `Packets::paginate`. ([PR](https://github.com/Serial-ATA/lofty-rs/pull/79))

## [0.3.1] - 2022-03-03

### Fixed
- Segment tables are written correctly with data spanning multiple pages ([issue](https://github.com/Serial-ATA/lofty-rs/issues/37))
