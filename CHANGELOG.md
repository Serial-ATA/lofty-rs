# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **ItemKey**:
  - `ItemKey::AlbumArtists`, available for ID3v2, Vorbis Comments, APE, and MP4 Ilst ([PR](https://github.com/Serial-ATA/lofty-rs/pull/523))
      - This is a multi-value item that stores each artist for a track. It should be retrieved with `Tag::get_strings` or `Tag::take_strings`.
      - For example, a track has `ItemKey::TrackArtist` = "Foo & Bar", then `ItemKey::AlbumArtists` = ["Foo", "Bar"].
  - `ItemKey::UnsyncLyrics` ([issue](https://github.com/Serial-ATA/lofty-rs/issues/561)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/568))
    - In formats like Vorbis Comments, `ItemKey::Lyrics` may actually contain synchronized lyrics in LRC format. To help with the ambiguity, some
      apps may write a separate field containing normal, unsynchronized lyrics.
    - In other formats where the difference doesn't matter (like ID3v2), this will act exactly the same as `ItemKey::Lyrics`.
  - `ItemKey::ReleaseCountry` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/573))
    - Currently, this maps to the [fields](https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#id30) used by MusicBrainz Picard, which expect an
      ISO 3166-1 code.
  - `ItemKey::AcoustId` and `ItemKey::AcoustIdFingerprint` ([issue](https://github.com/Serial-ATA/lofty-rs/issues/455)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/575))
    - These two fields come from [AcoustID], and can appear multiple times in a single tag.
  - `ItemKey::Description` mapping for Vorbis Comments ([issue](https://github.com/Serial-ATA/lofty-rs/issues/585)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/587))
- **Serde**: [Serde] support for `*Type` enums (`FileType`, `TagType`, `PictureType`) ([issue](https://github.com/Serial-ATA/lofty-rs/issues/533)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/534))
  - Support can be enabled with the new `serde` feature (not enabled by default)
- **Probe**: `Probe::read_bound()` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/557))
  - Same as `Probe::read()`, but returns a [`BoundTaggedFile`](https://docs.rs/lofty/latest/lofty/file/struct.BoundTaggedFile.html)
- **ID3v2** ([PR](https://github.com/Serial-ATA/lofty-rs/pull/576)):
  - Unified the two generic conversion paths
    - The background conversion used in `Tag::save_to()`, and the direct conversion done via `Into::<Id3v2Tag>::into(tag)` used to
      take different paths, causing certain conversions and frame merging to not occur in the former case ([issue](https://github.com/Serial-ATA/lofty-rs/issues/349)).
      They now use the same logic, which has also been rewritten to reuse data whenever possible, instead of cloning like before.
  - The following frames now use `Cow` internally: `CommentFrame`, `UnsynchronizedTextFrame`, `TextInformationFrame`, `ExtendedTextFrame`, `UrlLinkFrame`, `ExtendedUrlFrame`,
    `AttachedPictureFrame`, `PopularimeterFrame`, `KeyValueFrame`, `RelativeVolumeAdjustmentFrame`, `UniqueFileIdentifierFrame`, `OwnershipFrame`, `EventTimingCodesFrame`,
    `PrivateFrame`, `BinaryFrame`
  - `FrameId::is_valid()` and `FrameId::is_outdated()`
- **WriteOptions**: `WriteOptions::lossy_text_encoding()` to replace invalid characters when encoding strings ([PR](https://github.com/Serial-ATA/lofty-rs/pull/594))
  - When enabled, any non-representable character will be replaced with `?` (e.g. `lфfty` in `TextEncoding::Latin1` will return `l?fty`)
- **Popularimeter**: Generic user-specified star rating support ([discussion](https://github.com/Serial-ATA/lofty-rs/discussions/581)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/597))
  - ***These items require special handling.***
    See [the docs](https://docs.rs/lofty/latest/lofty/tag/items/popularimeter/index.html) for more details.
- **Other**: `EXTENSIONS` list containing common file extensions for all supported audio file types ([issue](https://github.com/Serial-ATA/lofty-rs/issues/509)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/558))
  - This is useful for filtering files when scanning directories. If your app uses extension filtering, **please consider switching to this**, as to not
    miss any supported files.

### Changed
- **ID3v2**: Check `TXXX:ALBUMARTIST` and `TXXX:ALBUM ARTIST` for `ItemKey::AlbumArtist` conversions
- **ID3v1**:
  - The `year` field in `Id3v1Tag` is now a `u16`, instead of a `String` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/574))
  - Strings are now verified to be Latin-1 ([PR](https://github.com/Serial-ATA/lofty-rs/pull/594))
- **Vorbis Comments**: Check `ALBUM ARTIST` for `ItemKey::AlbumArtist` conversions
- **Vorbis Comments**: Support `DISCNUMBER` fields with the `current/total` format. ([issue](https://github.com/Serial-ATA/lofty-rs/issues/543)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/544))
    - These fields will now properly be split into `DISCNUMBER` and `DISCTOTAL`, making it possible to use them with
      [Accessor::disk()](https://docs.rs/lofty/latest/lofty/tag/trait.Accessor.html#method.disk) and [Accessor::disk_total()](https://docs.rs/lofty/latest/lofty/tag/trait.Accessor.html#method.disk_total).
* **ItemKey**:
  * `ItemKey` is now `Copy` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/526))
  * `ItemKey::Popularimeter` is now intended to be used with the new `Popularimeter` type ([PR](https://github.com/Serial-ATA/lofty-rs/pull/597))
* **FileType**: Replaced `FileType::supports_tag_type()` with `FileType::tag_support()` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/566))
  * Rather than a simple `bool`, this now returns a `TagSupport`, which can describe three states: unsupported, read-only, and read/write
* **TaggedFileExt**: Replaced `TaggedFileExt::supports_tag_type()` with `TaggedFileExt::tag_support()` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/566))
* **FileResolver**: Replaced `FileResolver::supported_tag_types()` with `FileResolver::tag_support()` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/566))

### Fixed
- **ID3v2**:
  - Support parsing UTF-16 `COMM`/`USLT` frames with a single BOM ([issue](https://github.com/Serial-ATA/lofty-rs/issues/532)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/535))
    - Some encoders will only write a BOM to the frame's description, rather than to every string in the frame.
      This was previously only supported in `SYLT` frames, and has been extended to `COMM` and `USLT`.
  - Don't error on empty `SYLT` strings ([issue](https://github.com/Serial-ATA/lofty-rs/issues/563)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/564))
- **Vorbis Comments**: Parse `TRACKNUMBER` with respect to `ParseOptions::implicit_conversions` ([issue](https://github.com/Serial-ATA/lofty-rs/issues/540)) ([PR](https://github.com/Serial-ATA/lofty-rs/issues/542))
- **APE**: Fix disc number removal/writing ([issue](https://github.com/Serial-ATA/lofty-rs/issues/545)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/546))
- **AAC/ADTS**: Fix frame header search ([issue](https://github.com/Serial-ATA/lofty-rs/issues/584)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/586))
  - When searching for the next frame, the parser was not fully skipping the previous one. If the AAC payload contained the frame sync bits and an otherwise invalid ADTS
    header, then the parser would error.
- **FLAC**: Fix corruption of files with no metadata blocks ([issue](https://github.com/Serial-ATA/lofty-rs/issues/549)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/583))

### Removed

* **ItemKey**: `ItemKey::Unknown` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/526))
    * `Tag` is now intended for generic metadata editing only, with format-specific items only being available through concrete tag types.
      See <https://github.com/Serial-ATA/lofty-rs/issues/521> for the rationale.
* **Picture**: `Picture::new_unchecked()`, replaced with `Picture::unchecked()` returning a builder ([issue](https://github.com/Serial-ATA/lofty-rs/issues/468)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/569))
* **Accessor**: `Accessor::*_year()` methods, replaced with `Accessor::*_date()` ([issue](https://github.com/Serial-ATA/lofty-rs/issues/565)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/574))
  - Since all formats (*except ID3v1*) have full date support, the generic API now accepts `Timestamp`s. For ID3v1, the date will be truncated
    down to the year for conversions/writing.
  - Year tags can still be set manually with `ItemKey::Year`
- **ID3v2**: `AttachedPictureFrame::as_bytes()` no longer supports encoding ID3v2.2 `PIC` frames

## [0.22.4] - 2025-04-29

### Changed

* **MP4**:
  * A missing `mdat` atom is no longer a hard error when reading properties ([PR](https://github.com/Serial-ATA/lofty-rs/pull/515))
    * This is now only an error in `Strict` mode. Note that any properties read in a file with no `mdat` atom are essentially useless.
  * Incorrectly sized `free` atoms are no longer a hard error ([PR](https://github.com/Serial-ATA/lofty-rs/pull/516))
    * If a `free` atom claims to be larger than the remainder of the stream, parsing will simply stop. This will now only
      be a `SizeMismatch` error in `Strict` mode. Invalid padding is a common issue in all tag formats due to buggy software,
      so it's better to work around it by default rather than discard the entire stream as invalid.
* **WAV**:
  * When writing tags, the writer will be constrained to the stream size reported by the file, not by the file's actual length ([PR](https://github.com/Serial-ATA/lofty-rs/pull/517))
    * Previously, tags were simply written to the end of the file, but this would break files that have junk data appended.
    * This allows for files with appended junk data that falls outside of the stream length. This can be caused by buggy software
      misusing padding.

## [0.22.3] - 2025-04-04

### Added

* **MimeType**: `MimeType::ext()` to get the standard file extension of `Picture` `MimeType`s ([PR](https://github.com/Serial-ATA/lofty-rs/pull/510))

### Changed

* **2024 Edition**: Set the edition to 2024 and MSRV to 1.85 ([issue](https://github.com/Serial-ATA/lofty-rs/issues/35)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/513))
  * This is not a breaking change, as there was no MSRV prior.

## [0.22.2] - 2025-02-08

Thanks, [@Lepidopteran](https://github.com/Lepidopteran) for this release!

### Fixed

* **Docs** ([PR](https://github.com/Serial-ATA/lofty-rs/pull/504)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/506)): Updated old (invalid) links
* **ID3v2** ([issue](https://github.com/Serial-ATA/lofty-rs/issues/507)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/508)): Preserve all values of `ItemKey::MusicBrainz{ArtistId, ReleaseArtistId, WorkId}`
  * Previously, the fields would be written with only the *last* value in the list.

## [0.22.1] - 2025-01-11

### Changed
- **VorbisComments**: Support `TRACKNUMBER` fields with the `current/total` format. ([issue](https://github.com/Serial-ATA/lofty-rs/issues/493)) ([issue](https://github.com/Serial-ATA/lofty-rs/issues/499)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/500))
  - These fields will now properly be split into `TRACKNUMBER` and `TRACKTOTAL`, making it possible to use them with 
    [Accessor::track()](https://docs.rs/lofty/latest/lofty/tag/trait.Accessor.html#method.track) and [Accessor::track_total()](https://docs.rs/lofty/latest/lofty/tag/trait.Accessor.html#method.track_total).

## [0.22.0] - 2025-01-05

### Added
- **ItemKey**: `ItemKey::TrackArtists`, available for ID3v2, Vorbis Comments, APE, and MP4 Ilst ([PR](https://github.com/Serial-ATA/lofty-rs/pull/454))
  - This is a multi-value item that stores each artist for a track. It should be retrieved with `Tag::get_strings` or `Tag::take_strings`.
  - For example, a track has `ItemKey::TrackArtist` = "Foo & Bar", then `ItemKey::TrackArtists` = ["Foo", "Bar"].
  - See <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#artists>
- **UnsynchronizedStream**: `UnsynchronizedStream::get_ref()` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/459))
- **Ilst** ([PR](https://github.com/Serial-ATA/lofty-rs/pull/461)):
  - Methods to quickly set/check boolean flags:
    - `Ilst::set_flag`
    - `Ilst::is_podcast`
    - `Ilst::is_compilation`
    - `Ilst::is_gapless`
    - `Ilst::is_show_work`
    - `Ilst::is_hd_video`
  - `DataType` enum
    - Previously, atom data types were stored as a `u32`, with their names being available in `mp4::constants`.
      Now, instead of `mp4::constants::BE_SIGNED_INTEGER`, you can use `DataType::BeSignedInteger`, for example.
    - It can be converted to and from a `u32`
  - `AtomData::data_type()` to get the data type code of the atom content.

### Changed
- **Ilst**:
  - Add new rules for `gnre` atom upgrades ([issue](https://github.com/Serial-ATA/lofty-rs/issues/409)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/485))
    - In the case that a `©gen` and `gnre` atom are present in a file, there was no way to tell which `©gen` atoms were upgraded.
      the new rules are:
      - `gnre` present + no `©gen` present, `gnre` gets upgraded as normal
      - `gnre` present + `©gen` present, `©gen` takes precedence and `gnre` is discarded
        - With [ParsingOptions::implicit_conversions](https://docs.rs/lofty/latest/lofty/config/struct.ParseOptions.html#method.implicit_conversions)
          set to `false`, `gnre` will be retained as an atom of type `Unknown`.
  - Ignore invalid `covr` data types when not using `ParsingMode::Strict` ([issue](https://github.com/Serial-ATA/lofty-rs/issues/482)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/486))
- **RIFF INFO**: Ignore text decoding errors when not using `ParsingMode::Strict` ([issue](https://github.com/Serial-ATA/lofty-rs/issues/373)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/486))
  - RIFF INFO tags may be encoded with a non UTF-8 system encoding, that we have no way of knowing. It's no longer an error to read these files,
    it's just unlikely that anything useful come out of the RIFF INFO tags.

### Fixed
- **MusePack**: Fix potential panic when the beginning silence makes up the entire sample count ([PR](https://github.com/Serial-ATA/lofty-rs/pull/449))
- **Timestamp**:
  - Support timestamps without separators (ex. "20240906" vs "2024-09-06") ([issue](https://github.com/Serial-ATA/lofty-rs/issues/452)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/453))
  - `Timestamp::parse` will now short-circuit when possible in `ParsingMode::{BestAttempt, Relaxed}` ([issue](https://github.com/Serial-ATA/lofty-rs/issues/462)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/463))
    - For example, the timestamp "2024-06-03 14:08:49" contains a space instead of the required "T" marker.
      In `ParsingMode::Strict`, this would be an error. Otherwise, the parser will just stop once it hits the space
      and return the timestamp up to that point.
- **ID3v2**:
  - `ItemKey::Director` will now be written correctly as a TXXX frame ([PR](https://github.com/Serial-ATA/lofty-rs/issues/454))
  - When skipping invalid frames in `ParsingMode::{BestAttempt, Relaxed}`, the parser will no longer be able to go out of the bounds
    of the frame content ([issue](https://github.com/Serial-ATA/lofty-rs/issues/458)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/459))
- **MP4**: Support for flag items (ex. `cpil`) of any size (not just 1 byte) ([issue](https://github.com/Serial-ATA/lofty-rs/issues/457)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/460))
- **Fuzzing** (Thanks [@qarmin](https://github.com/qarmin)!) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/476)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/479)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/483)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/489)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/496)):
    - **MusePack**: Fix panic when ID3v2 tag sizes exceed the stream length ([issue](https://github.com/Serial-ATA/lofty-rs/issues/470))
    - **WAV**: Fix panic when calculating bit depth with abnormally large `bytes_per_sample` ([issue](https://github.com/Serial-ATA/lofty-rs/issues/471))
    - **WavPack***: Fix panic when encountering wrongly sized blocks ([issue](https://github.com/Serial-ATA/lofty-rs/issues/472)) ([issue](https://github.com/Serial-ATA/lofty-rs/issues/480))
    - **WavPack***: Fix panic when encountering zero-sized blocks ([issue](https://github.com/Serial-ATA/lofty-rs/issues/473))
    - **WavPack**: Verify the size of non-standard sample rate blocks ([issue](https://github.com/Serial-ATA/lofty-rs/issues/488))
    - **WavPack**: Fix potential overflow in bit depth calculation ([issue](https://github.com/Serial-ATA/lofty-rs/issues/491))
    - **MPEG**: Fix panic when APE tags are incorrectly sized ([issue](https://github.com/Serial-ATA/lofty-rs/issues/474))
    - **MPEG**: Fix panic when calculating the stream length for files with improperly sized frames ([issue](https://github.com/Serial-ATA/lofty-rs/issues/487))
    - **ID3v2**: Fix panic when parsing non-ASCII `TDAT` and `TIME` frames in `TDRC` conversion ([issue](https://github.com/Serial-ATA/lofty-rs/issues/477))
    - **APE**: Fix panic when parsing incorrectly sized header APE tags ([issue](https://github.com/Serial-ATA/lofty-rs/issues/481))

## [0.21.1] - 2024-08-28

### Changed
- **FLAC**: Vendor strings are now retained when writing tags ([PR](https://github.com/Serial-ATA/lofty-rs/pull/443))
  - This behavior already exists for OGG formats.

### Fixed
- **FLAC**: Stop writing invalid `PADDING` blocks ([issue](https://github.com/Serial-ATA/lofty-rs/issues/442)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/446))
  - If a `PADDING` block existed in the original file, and it wasn't placed at the end of the header, it would be
    moved without setting the `Last-metadata-block` flag. This would cause decoders to believe that the file was corrupted.
- **Fuzzing** (Thanks [@qarmin](https://github.com/qarmin)!) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/444)):
  - **MusePack**: Fix panic when tag sizes exceed the stream length ([issue](https://github.com/Serial-ATA/lofty-rs/issues/440))
  - **AAC**: Fix panic when tag sizes exceed the stream length ([issue](https://github.com/Serial-ATA/lofty-rs/issues/439))

## [0.21.0] - 2024-07-29

### Added
- **ParseOptions**:
  - `ParseOptions::read_tags` to skip the parsing of tags ([issue](https://github.com/Serial-ATA/lofty-rs/issues/251)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/406))
  - `ParseOptions::read_cover_art` ([issue](https://github.com/Serial-ATA/lofty-rs/issues/186)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/415))
    - As cover art can be large, it is now possible to disable reading it when parsing a file.
  - `ParseOptions::implicit_conversions` to prevent automatic data conversions ([PR](https://github.com/Serial-ATA/lofty-rs/pull/411))
    - Be sure to read the warnings in the docs to understand what this means.
- **ID3v2**: Support writing ID3v2.3 tags ([issue](https://github.com/Serial-ATA/lofty-rs/issues/62)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/411))
    - This can be done by setting `WriteOptions::use_id3v23` to `true`.
- **Tag**: `Tag::take_filter` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/414))
  - This is like `Tag::take`, but allows for per-`TagItem` filtering.
  - This is useful for `TagType::Id3v2`, as it allows specifying descriptions and languages for frames.
    See the docs and PR description for more details.

### Changed
- **Timestamp**: `Timestamp::parse` with empty inputs will return `None` when not using `ParsingMode::Strict` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/416))
- **MP4**: Atoms with sizes greater than the remaining file size will be ignored with `ParsingMode::Relaxed` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/433))
- **ID3v2** ([PR](https://github.com/Serial-ATA/lofty-rs/pull/437)):
  - `PopularimeterFrame::as_bytes()` is now fallible
  - `PrivateFrame::as_bytes()` is now fallible

### Fixed
- **Fuzzing** (Thanks [@qarmin](https://github.com/qarmin)!) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/423)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/434)):
  - **MP4**:
    - Fix panic when reading properties of a file with no timescale specified ([issue](https://github.com/Serial-ATA/lofty-rs/issues/418))
    - Fix panics when reading improperly sized freeform atom identifiers ([issue](https://github.com/Serial-ATA/lofty-rs/issues/425)) ([issue](https://github.com/Serial-ATA/lofty-rs/issues/426))
    - Fix panic when `data` atom length is less than 16 bytes ([issue](https://github.com/Serial-ATA/lofty-rs/issues/429))
    - Fix panic with improperly sized freeform identifiers ([issue](https://github.com/Serial-ATA/lofty-rs/issues/430))
    - Fix panic when `hdlr` atom is an unexpected length ([issue](https://github.com/Serial-ATA/lofty-rs/issues/435))
    - Fix panic when `stts` atom has an unrealistically large entry count ([issue](https://github.com/Serial-ATA/lofty-rs/issues/436)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/437))
  - **WAV**:
    - Fix panic when reading properties with large written bytes per second ([issue](https://github.com/Serial-ATA/lofty-rs/issues/420))
    - Fix panic when reading an improperly sized INFO LIST ([issue](https://github.com/Serial-ATA/lofty-rs/issues/427))
    - Fix panic when reading a fmt chunk with an invalid bits_per_sample field ([issue](https://github.com/Serial-ATA/lofty-rs/issues/428))
  - **Vorbis**:
    - Fix panic when reading properties of a file with large absolute granule positions ([issue](https://github.com/Serial-ATA/lofty-rs/issues/421))
    - Fix attempted large allocations with invalid comment counts ([issue](https://github.com/Serial-ATA/lofty-rs/issues/419))
  - **FLAC**: Fix panic when reading properties of a file with incorrect block sizes ([issue](https://github.com/Serial-ATA/lofty-rs/issues/422))
  - **AIFF**: Fix panic when reading properties of a file with invalid f80 sample rate ([issue](https://github.com/Serial-ATA/lofty-rs/issues/424))

## [0.20.1] - 2024-07-02

### Fixed
- **MPEG**: Fix durations being slightly off ([issue](https://github.com/Serial-ATA/lofty-rs/issues/412)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/413))

## [0.20.0] - 2024-06-06

### Added
- **Tag**:
  - Support `ItemKey::ParentalAdvisory` for `Ilst` and `Id3v2Tag` ([issue](https://github.com/Serial-ATA/lofty-rs/issues/99)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/388))
    - This will allow for generic edits to the iTunes-style parental advisory tag. Note that this will use the
      numeric representation. For more information, see: https://docs.mp3tag.de/mapping/#itunesadvisory.
  - New `tag::items` module for generic representations of complex tag items
  - New `Timestamp` item for ISO 8601 timestamps ([PR](https://github.com/Serial-ATA/lofty-rs/pull/389))
- **ID3v2**: Special handling for frames with timestamps with `Frame::Timestamp` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/389))
- **GlobalOptions**: `preserve_format_specific_items()` ([issue](https://github.com/Serial-ATA/lofty-rs/issues/302)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/391))
  - This will allow for the preservation of format-specific items when converting between tag types.
  - Previously, these items would be discarded when converting to the generic `Tag`. Now they are stored
    in an immutable container, and silently rejoined with the tag when converting back to the original format
    or when writing.
- **TagItem**: `set_lang` and `set_description` to allow for generic conversions of additional ID3v2 frames (such as comments) ([issue](https://github.com/Serial-ATA/lofty-rs/issues/383)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/392))
- **BoundTaggedFile**: `BoundTaggedFile::into_inner` to get the original file handle ([PR](https://github.com/Serial-ATA/lofty-rs/pull/404))

### Changed
- **VorbisComments**/**ApeTag**: Verify contents of `ItemKey::FlagCompilation` during `Tag` merge ([PR](https://github.com/Serial-ATA/lofty-rs/pull/387))
- **ID3v2**:
  - ⚠️ Important ⚠️: `Frame` has been converted to an `enum` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/390)):
    - This makes it easier to validate frame contents, as one can no longer make an `AttachedPictureFrame` with the ID `"TALB"`, for example.
      See the PR for a full description of the changes.
    ```rust
    // Old:
    let frame = Frame::new(
        "TALB",
        FrameType::Text(TextInformationFrame {
            TextEncoding::UTF8,
            value: String::from("Foo album"),
        }),
        FrameFlags::default(),
    ).unwrap();
    
    // New:
    let frame = Frame::Text(TextInformationFrame::new(
        FrameId::new("TALB").unwrap(),
        FrameFlags::default(),
        TextEncoding::UTF8,
        String::from("Foo album"),
    ));
    ```
  - Renamed `Popularimeter` -> `PopularimeterFrame`
  - Renamed `SynchronizedText` -> `SynchronizedTextFrame`

### Fixed
- **ID3v2**: Disallow 4 character TXXX/WXXX frame descriptions from being converted to `ItemKey` ([issue](https://github.com/Serial-ATA/lofty-rs/issues/309)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/394))
- **MPEG**: Durations estimated by bitrate are more accurate ([PR](https://github.com/Serial-ATA/lofty-rs/pull/395))
- **MP4**:
  - Bitrate calculation is now more accurate ([PR](https://github.com/Serial-ATA/lofty-rs/pull/398))
  - Existing tags will no longer be overridden if another `udta` atom is encountered ([PR](https://github.com/Serial-ATA/lofty-rs/pull/405))
- **WAV**: Bitrate calculation is now more accurate ([PR](https://github.com/Serial-ATA/lofty-rs/pull/399))
- **MusePack**: Overall improved audio properties  ([PR](https://github.com/Serial-ATA/lofty-rs/pull/402))

## [0.19.2] - 2024-04-26

### Added
- **Length**: `impl<T: Length> Truncate for &mut T`

## [0.19.1] - 2024-04-26 (YANKED)

### Added
- **Truncate**: `impl<T: Truncate> Truncate for &mut T` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/384))
- **Length**: `impl<T: Length> Truncate for &T` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/384))

### Changed
- **MP4**: All surrounding `free` atoms will be used when writing `ilst` tags ([issue](https://github.com/Serial-ATA/lofty-rs/issues/346)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/386))
  - Previously, only the `free` atoms immediately surrounding the `ilst` atom were used.

## [0.19.0] - 2024-04-21

### Added
- **WriteOptions** ([issue](https://github.com/Serial-ATA/lofty-rs/issues/228)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/363)):
	- ⚠️ Important ⚠️: This update introduces `WriteOptions` to allow for finer grained control over how
	  Lofty writes tags. These are best used as global user-configurable options, as most options will
	  not apply to all files. The defaults are set to be as safe as possible,
	  see [here](https://docs.rs/lofty/latest/lofty/struct.WriteOptions.html#impl-Default-for-WriteOptions).
- **Generic Writes** ([PR](https://github.com/Serial-ATA/lofty-rs/pull/290)):
  - ⚠️ Important ⚠️: This update introduces `FileLike`, which is a combination of the `Truncate` + `Length` traits
    that allows one to write to more than just `File`s. In short, `Cursor<Vec<u8>>` can now be written to.
- **ChannelMask**
  - `BitAnd` and `BitOr` implementations ([PR](https://github.com/Serial-ATA/lofty-rs/pull/371))
  - Associated constants for common channels, ex. `ChannelMask::FRONT_LEFT` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/371))
  - `ChannelMask::from_{mp4, opus}_channels`
- **Opus**: `OpusProperties` now contains the channel mask
- **AAC**: `AacProperties` now contains the channel mask
- **Prelude**: `lofty::prelude` module to make trait imports easier ([PR](https://github.com/Serial-ATA/lofty-rs/pull/374))

### Changed
- **ID3v2**: Ignore empty duplicate frames ([PR](https://github.com/Serial-ATA/lofty-rs/pull/351))
  - Some software will apparently write an empty duplicate frame after the actual frame. As the latest frame
    is the only one that gets preserved, we now check if the frame is empty before replacing.
- **Properties**: `FileProperties` and `ChannelMask` have been moved from the root to the new `lofty::properties`
                   module ([PR](https://github.com/Serial-ATA/lofty-rs/pull/372))
- **ParseOptions**/**WriteOptions**/**GlobalOptions**:
  - ⚠️ Important ⚠️: Moved to `lofty::config` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/374))
- **AudioFile**/**TaggedFileExt**/**TaggedFile**/**BoundTaggedFile**/**FileType**:
  - ⚠️ Important ⚠️: Moved to `lofty::file` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/374))
- **Tag**:
    - ⚠️ Important ⚠️- The following items have been moved to `lofty::tag` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/374)):
      - `Tag`
      - `Accessor`
      - `TagType`
      - `TagItem`
      - `ItemKey`
      - `ItemValue`
- **Probe**:
    - ⚠️ Important ⚠️- Moved to `lofty::probe` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/374)):
- **Picture**:
    - ⚠️ Important ⚠️- The following items have been moved to `lofty::picture` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/374)):
      - `Picture`
      - `PictureType`
      - `PictureInformation`
      - `MimeType`
- **IFF** ([PR](https://github.com/Serial-ATA/lofty-rs/pull/379)):
  - **AIFF**: `AIFFTextChunks` renamed to `AiffTextChunks`
  - **RIFF**: `RIFFInfoList` renamed to `RiffInfoList`

### Fixed
- **Vorbis**: Fix panic when reading properties of zero-length files ([issue](https://github.com/Serial-ATA/lofty-rs/issues/342)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/365))
- **ID3v2**:
  - Fix panic when reading a UTF-16 with no BOM ([issue](https://github.com/Serial-ATA/lofty-rs/issues/295)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/343))
  - Fix panic when reading an RVA2 frame with a peak larger than 248 bits ([issue](https://github.com/Serial-ATA/lofty-rs/issues/295)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/364))
- **WAV**: Length and bitrate values are properly rounded ([PR](https://github.com/Serial-ATA/lofty-rs/pull/367))
- **ParseOptions**: No longer derives `{PartialOrd, Ord}` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/369))

### ogg_pager
See [ogg_pager's changelog].

## [0.18.2] - 2024-01-23

### Fixed
- **MP4**: Padding for shrinking tags will no longer overwrite unrelated data ([PR](https://github.com/Serial-ATA/lofty-rs/pull/346))

## [0.18.1] - 2024-01-20 (YANKED)

### Fixed
- **ID3v2**: Fix panic in UTF-16 parsing when BOM is missing ([issue](https://github.com/Serial-ATA/lofty-rs/issues/295)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/343))
- **MP4**:
  - Properly handle track/disc numbers greater than 16 bits ([PR](https://github.com/Serial-ATA/lofty-rs/pull/341))
  - Atom offset updates will now be properly handled for shrinking tags ([PR](https://github.com/Serial-ATA/lofty-rs/pull/344))

## [0.18.0] - 2024-01-12

### Added
- **MP4**: Check if audio streams are DRM protected, exposed as `Mp4Properties::is_drm_protected()` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/297))
- **ID3v2**:
  - Add `Id3v2ErrorKind::EmptyFrame` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/299))
  - Support converting some TIPL frame values into generic `TagItem`s ([PR](https://github.com/Serial-ATA/lofty-rs/pull/301))
    - Supported TIPL keys are: "producer", "arranger", "engineer", "DJ-mix", "mix".
- **GlobalOptions**: Options local to the thread that persist between reads and writes ([PR](https://github.com/Serial-ATA/lofty-rs/pull/321))
  - See [the docs](https://docs.rs/lofty/latest/lofty/struct.GlobalOptions.html) for more information
- **ItemKey**: `ItemKey::IntegerBpm` ([issue](https://github.com/Serial-ATA/lofty-rs/issues/334)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/335))

### Changed
- **ID3v1**: Renamed `GENRES[14]` to `"R&B"` (Previously `"Rhythm & Blues"`) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/296))
- **MP4**: Duration milliseconds are now rounded to the nearest whole number ([PR](https://github.com/Serial-ATA/lofty-rs/pull/298))
- **ID3v2**:
  - Stop erroring on empty frames when not using `ParsingMode::Strict` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/299))
  - Verify contents of flag items (`ItemKey::FlagCompilation`, `ItemKey::FlagPodcast`) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/336))
  - `Id3v2Tag::get_text()` will now return the raw, unedited string ([PR](https://github.com/Serial-ATA/lofty-rs/pull/336))
    - Previously, all null separators were replaced with `"/"` to make the string easier to display.
      Now, null separators are only replaced in [`Accessor`](https://docs.rs/lofty/latest/lofty/trait.Accessor.html) methods.
      It is up to the caller to decide how to handle all other strings.
- **resolve**: Custom resolvers will now be checked before the default resolvers ([PR](https://github.com/Serial-ATA/lofty-rs/pull/319))
- **MPEG**: Up to `max_junk_bytes` will now be searched for tags between the start of the file and the first MPEG frame ([PR](https://github.com/Serial-ATA/lofty-rs/pull/320))
  - This allows us to read and write ID3v2 tags that are preceeded by junk
- **ItemKey**:
  - Renamed `ItemKey::PodcastReleaseDate` to `ItemKey::ReleaseDate` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/329))
  - Renamed `ItemKey::{PodcastURL, PoddcastGlobalUniqueID}` to `ItemKey::{PodcastUrl, PoddcastGlobalUniqueId}` ([issue](https://github.com/Serial-ATA/lofty-rs/issues/327)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/332)) 

### Fixed
- **MP4**:
  - The `dfLa` atom for FLAC streams will now be found, providing better properties ([PR](https://github.com/Serial-ATA/lofty-rs/pull/298))
  - Offset atoms (`stco`, `co64`, and `tfhd`) will now be updated when writing ([issue](https://github.com/Serial-ATA/lofty-rs/issues/308)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/318))
  - `ItemKey::FlagPodcast` will be checked in `Tag` -> `Ilst` conversion ([PR](https://github.com/Serial-ATA/lofty-rs/pull/336))
- **ID3v2**: Support UTF-16 encoded TIPL frames with a single BOM ([issue](https://github.com/Serial-ATA/lofty-rs/issues/306)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/307))
- **Speex**: Estimate bitrate when the nominal bitrate is not available ([PR](https://github.com/Serial-ATA/lofty-rs/pull/322))
  - When no nominal bitrate was provided, the bitrate was previously set to 0. Now we will give an estimate based
    on the stream length, which may or may not be entirely accurate.

### Removed
- **ItemKey**: `ItemKey::InvolvedPeople`
- **MimeType**: `MimeType::None`, `Picture` now stores an `Option<MimeType>`.
- **ID3v2**: `TextSizeRestrictions::None` and `ImageSizeRestrictions::None`
	- `TagRestrictions` now stores an `Option<TextSizeRestrictions>` and `Option<ImageSizeRestrictions>`.
- **MPEG**: `Emphasis::None`, `MpegProperties` now stores an `Option<Emphasis>`.

## [0.17.1] - 2023-11-26

### Changed
- **MP4**: Skip over invalid `ilst` atoms by default ([issue](https://github.com/Serial-ATA/lofty-rs/issues/291)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/292))

## [0.17.0] - 2023-11-14

### Added
- **ParseOptions**: `ParseOptions::allocation_limit` to change the default allocation limit. ([PR](https://github.com/Serial-ATA/lofty-rs/pull/276))
- **ID3v2**: `Id3v2Tag::genres` to handle all the ways genres can be stored in `TCON` frames. ([PR](https://github.com/Serial-ATA/lofty-rs/pull/286))

### Changed
- **VorbisComments**: When converting from `Tag` to `VorbisComments`, `ItemKey::Unknown`s will be checked for spec compliance. ([PR](https://github.com/Serial-ATA/lofty-rs/pull/272))
- **ID3v2**: Any trailing null terminators will be trimmed when reading Comment, Text, UserText, UserUrl, and UnsynchronizedText frames. ([PR](https://github.com/Serial-ATA/lofty-rs/pull/275))
- **Alloc**: The default allocation limit for any single tag item is now **16MB**. ([PR](https://github.com/Serial-ATA/lofty-rs/pull/276))
- **Probe**: `Probe::set_file_type()` will return the `Probe` to allow for builder-style usage. ([PR](https://github.com/Serial-ATA/lofty-rs/pull/276))

### Fixed
- **MP4**: Verify atom identifiers fall within a subset of characters ([PR](https://github.com/Serial-ATA/lofty-rs/pull/267))
  - For a multitude of reasons, garbage data can be left at the end of an atom, resulting in Lofty attempting to
    parse it as another atom definition. As the specification is broad, there is no way for us to say *with certainty*
    that an identifier is invalid. Now we unfortunately have to guess the validity based on the commonly known atoms.
    For this, we follow [TagLib]'s [checks](https://github.com/taglib/taglib/blob/b40b834b1bdbd74593c5619e969e793d4d4886d9/taglib/mp4/mp4atom.cpp#L89).
- **ID3v1**: No longer error on inputs shorter than 128 bytes (the length of an ID3v1 tag). ([PR](https://github.com/Serial-ATA/lofty-rs/pull/270))
- **ID3v2**: No longer error on multi-value UTF-16 encoded text frames ([issue](https://github.com/Serial-ATA/lofty-rs/issues/265)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/284))

### Removed
- **MP4**: `Ilst::{track_total, disc_number, disc_total}` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/269))
  - These existed prior to the methods on `Accessor`. There is no need to keep them around, as they behave the same.

## [0.16.1] - 2023-10-15

### Fixed
- **MP4**: Skip unexpected or empty data atoms in ilst ([PR](https://github.com/Serial-ATA/lofty-rs/pull/261))
  - It is possible for an `ilst` item to have both empty `data` atoms and unexpected (likely vendor-specific) atoms other than `data`.
    These are both cases we can safely ignore unless using `ParsingMode::Strict`.

## [0.16.0] - 2023-10-01

### Added
- **ID3v2**:
  - Support for "RVA2", "OWNE", "ETCO", and "PRIV" frames through
               `id3::v2::{RelativeVolumeAdjustmentFrame, OwnershipFrame, EventTimingCodesFrame, PrivateFrame}` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/236))
  - `FrameId` now implements `Display` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/257))
  - `Id3v2Tag::get_texts` for multi-value text frames ([PR](https://github.com/Serial-ATA/lofty-rs/pull/257))
- **MP4** ([PR](https://github.com/Serial-ATA/lofty-rs/pull/241)):
  - `Atom::into_data`
  - `Atom::merge`
- **OGG**: Support for reading "COVERART" fields, an old deprecated image storage format. ([issue](https://github.com/Serial-ATA/lofty-rs/issues/253)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/254))

### Changed
- **ID3v2**:
  - Tag header parsing errors will be ignored unless using `ParsingMode::Strict` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/214))
  - For spec compliance, `Id3v2Tag::insert` will now check for frames that are only meant to appear
  in a tag once and remove them. Those frames are: "MCDI", "ETCO", "MLLT", "SYTC", "RVRB", "PCNT", "RBUF", "POSS", "OWNE", "SEEK", and "ASPI". ([PR](https://github.com/Serial-ATA/lofty-rs/pull/236))
  - `Id3v2Tag::remove` will now take a `FrameId` rather than `&str` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/237))
  - `FrameId` now implements `Into<Cow<'_, str>>`, making it possible to use it in `Frame::new` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/237))
  - `Id3v2Tag` getters will now use `&FrameId` instead of `&str` for IDs ([PR](https://github.com/Serial-ATA/lofty-rs/pull/257))
- **MP4** ([PR](https://github.com/Serial-ATA/lofty-rs/pull/241)):
  - `Ilst::remove` will now return all of the removed atoms
  - `Ilst::insert_picture` will now combine all pictures into a single `covr` atom
  - `Ilst::insert` will now merge atoms with the same identifier into a single atom
- **FLAC**:
  - Allow multiple Vorbis Comment blocks when not using `ParsingMode::Strict` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/242))
    - This is not allowed [by spec](https://xiph.org/flac/format.html#def_VORBIS_COMMENT), but is still possible
      to encounter in the wild. Now we will just take whichever tag happens to be latest in the stream and
      use it, they **will not be merged**.
  - Allow picture types greater than 255 when not using `ParsingMode::Strict` ([issue](https://github.com/Serial-ATA/lofty-rs/issues/253)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/254))
    - This is not allowed [by spec](https://xiph.org/flac/format.html#metadata_block_picture), but has been encountered
      in the wild. Now we will just cap the picture type at 255.

### Fixed
- **WavPack**: Custom sample rates will no longer be overwritten ([PR](https://github.com/Serial-ATA/lofty-rs/pull/244))
  - When a custom sample rate (or multiplier) was encountered, it would accidentally be overwritten with 0, causing
    incorrect duration and bitrate values.
- **APE**: Reading properties on older files will no longer error ([PR](https://github.com/Serial-ATA/lofty-rs/pull/245))
  - Older APE stream versions were not properly handled, leading to incorrect properties and errors.
- **ID3v2**: Don't expect text frames to be null terminated ([issue](https://github.com/Serial-ATA/lofty-rs/issues/255)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/256))

## [0.15.0] - 2023-07-11

### Added
- **ID3v2**:
  - `Id3v2ErrorKind::UnsupportedFrameId` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/212))
  - `FrameValue::KeyValue` for TIPL/TMCL frames ([PR](https://github.com/Serial-ATA/lofty-rs/pull/214))
  - `Id3v2Tag::get_user_text`, `Id3v2Tag::insert_user_text`, and `Id3v2Tag::remove_user_text` for working with TXXX frames ([PR](https://github.com/Serial-ATA/lofty-rs/pull/232))
- **ParseOptions**: `ParseOptions::max_junk_bytes`, allowing the parser to sift through junk bytes to find required information, rather than
                    immediately declare a file invalid. ([discussion](https://github.com/Serial-ATA/lofty-rs/discussions/219)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/227))
- **WavPack**: `WavPackProperties` now contains the channel mask, accessible through `WavPackProperties::channel_mask()` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/230))
- **AIFF**:
  - `AiffProperties` to hold additional AIFF-specific information
  - AIFC compression types are now exposed through `AiffCompressionType`

### Changed
- **ID3v2**:
  - `Id3v2ErrorKind::BadFrameId` now contains the frame ID ([PR](https://github.com/Serial-ATA/lofty-rs/pull/212))
  - Bad frame IDs will no longer error when using `ParsingMode::{Relaxed, BestAttempt}`. The parser will now just move on to the next frame. ([PR](https://github.com/Serial-ATA/lofty-rs/pull/214))
- **APE**: The default track/disk number is now `0` to line up with ID3v2.
           This is only used when `set_{track, disk}_total` is used without a corresponding `set_{track, disk}`.
- **VorbisComments**: When writing, items larger than `u32::MAX` will throw `ErrorKind::TooMuchData`, rather than be silently discarded.
- **AIFF**: `AiffFile` will no longer use `FileProperties`. It now uses `AiffProperties`.

### Fixed
- **APE**: Track/Disk number pairs are properly converted when writing ([issue](https://github.com/Serial-ATA/lofty-rs/issues/159)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/216))
- **ID3v2**: TIPL/TMCL frames will no longer be read as a single terminated string ([issue](https://github.com/Serial-ATA/lofty-rs/pull/213)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/214))
- **WavPack**: Multichannel files will no longer be marked as mono, supporting up to 4095 channels ([PR](https://github.com/Serial-ATA/lofty-rs/pull/230))

## [0.14.0] - 2023-06-08

### Added
- **ParsingMode**: A new variant, `BestAttempt` will attempt to fill holes in otherwise valid tag items ([PR](https://github.com/Serial-ATA/lofty-rs/pull/205))
- **🎉 Support for Musepack files** ([issue](https://github.com/Serial-ATA/lofty-rs/issues/199)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/200))

### Changed
- **Probe**: The default `ParsingMode` is now `ParsingMode::BestAttempt` (It was previously `ParsingMode::Strict`)
- **Alloc**:
  - ⚠️ Important ⚠️: The allocation limit for any single tag item is now **8MB** ([PR](https://github.com/Serial-ATA/lofty-rs/pull/207)).
                   This is not configurable yet ([issue](https://github.com/Serial-ATA/lofty-rs/issues/208)).

### Fixed
- **MP4**: Fixed potential panic with malformed `plID` atoms ([issue](https://github.com/Serial-ATA/lofty-rs/issues/201)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/202))

### Removed
- **ID3v2**: Removed `id3::util::synchsafe::unsynch_content`. This has been replaced with [UnsynchronizedStream](https://docs.rs/lofty/latest/lofty/id3/v2/util/synchsafe/struct.UnsynchronizedStream.html).

## [0.13.0] - 2023-05-08

### Added
- **Tag**/**ItemValue**: `Tag::remove_empty`/`ItemValue::is_empty` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/181))
- **ItemKey**: Variants for MusicBrainz Release group/Artist/Release artist/Work IDs ([PR](https://github.com/Serial-ATA/lofty-rs/pull/182))
- **ID3v2**:
  - `ItemKey::{Barcode, CatalogNumber}` mappings ([PR](https://github.com/Serial-ATA/lofty-rs/pull/183))
  - `SynchsafeInteger` trait to convert multiple integer types to/from their unsynchronized variants ([PR](https://github.com/Serial-ATA/lofty-rs/pull/187))
  - Concrete types for all `FrameValue` variants ([PR](https://github.com/Serial-ATA/lofty-rs/pull/184))
  - Support for the audio-text accessibility (ATXT) frame ([PR](https://github.com/Serial-ATA/lofty-rs/pull/188))
- **VorbisComments**: `ItemKey::Barcode` mapping ([PR](https://github.com/Serial-ATA/lofty-rs/pull/183))

### Changed
- **ID3v1**: `ID3v1Tag` -> `Id3v1Tag`
- **ID3v2**:
  - `SyncTextInformation` no longer uses a String for its language ([PR](https://github.com/Serial-ATA/lofty-rs/pull/184))
  - `FrameID` -> `FrameId`
  - `ID3v2Version` -> `Id3v2Version`
  - `ID3v2TagFlags` -> `Id3v2TagFlags`
  - `ID3v2Tag` -> `Id3v2Tag`
  - There are fewer redundant, intermediate allocations ([PR](https://github.com/Serial-ATA/lofty-rs/pull/194))
- **FileType/TagType/ItemKey**: All variants have been changed to UpperCamelCase ([PR](https://github.com/Serial-ATA/lofty-rs/pull/190))
- **MPEG**:
  - `MPEGFile` -> `MpegFile`
  - `MPEGProperties` -> `MpegProperties`

### Fixed
- **ID3v2**: Compressed frames are now properly handled ([PR](https://github.com/Serial-ATA/lofty-rs/pull/191))

### Removed
- **ID3v2**:
  - All uses of `ID3v2ErrorKind::Other` have been replaced with concrete errors
  - `SyncTextInformation` and `GEOBInformation` have been flattened into their respective items ([PR](https://github.com/Serial-ATA/lofty-rs/pull/196))

## [0.12.1] - 2023-04-10

### Fixed
- **WAV**: Fix division by zero when reading the properties of an empty stream ([issue](https://github.com/Serial-ATA/lofty-rs/issues/174)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/175))
- **ID3v2** ([PR](https://github.com/Serial-ATA/lofty-rs/pull/177)):
  - Export `id3::v2::UniqueFileIdentifierFrame`
  - Export `id3::v2::Popularimeter`

## [0.12.0] - 2023-04-04

### Added
- **Properties**: Expose channel mask (only supported for WAV and MPEG for now) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/155))
- **ItemKey**: `InitialKey` mapping for Vorbis Comments ([PR](https://github.com/Serial-ATA/lofty-rs/pull/156))
- **VorbisComments**: `VorbisComments::push` to allow for a non-replacing insertion ([PR](https://github.com/Serial-ATA/lofty-rs/pull/169))
- **Tags**: `<Tag>::new()` as an alias for `<Tag>::default()` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/170))
- **Picture**: `Picture::into_data()` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/173))

### Changed
- **APE**/**ID3v1**/**ID3v2**/**Tag**:
  - Allow empty strings as values instead of removing the corresponding item when empty ([PR](https://github.com/Serial-ATA/lofty-rs/pull/134))
  - Separated the trait `SplitAndMergeTag` into `SplitTag` and `MergeTag` to prevent any unexpected or undefined behavior at runtime ([PR](https://github.com/Serial-ATA/lofty-rs/pull/143))
- **VorbisComments** ([PR](https://github.com/Serial-ATA/lofty-rs/pull/169)):
  - Keys will now be verified according to spec before insertion
  - Getters will now case-insensitively search for keys
  - `TRACKNUM` will now be considered in the `Accessor::*track` methods
- **Tags**: Method names are more consistent ([PR](https://github.com/Serial-ATA/lofty-rs/pull/171))

### Fixed
- **ID3v2**:
  - Fix conversion of user defined frames when using `Tag` writing interface ([issue](https://github.com/Serial-ATA/lofty-rs/issues/140)) ([PR](https://github.com/Serial-ATA/lofty-rs/issues/142))
  - Fix writing of tag/disk numbers when using `Tag` writing interface ([issue](https://github.com/Serial-ATA/lofty-rs/issues/145)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/149))
- **MP4** ([PR](https://github.com/Serial-ATA/lofty-rs/pull/163)):
  - Fix the incorrect size being written for newly created `moov.udta.meta` atoms
    - Previously, the 8 bytes for the size and identifier were not accounted for
  - The parser has been further restricted to avoid going out of bounds
    - This was only an issue if there was garbage data after the `moov` item *and* the parser had not yet found
      the `moov.udta` atom.
- **WavPack** ([PR](https://github.com/Serial-ATA/lofty-rs/pull/168)):
  - Fewer errors are suppressed
  - Metadata sub-blocks are now properly parsed
  - Bitrate calculation will now properly round down

## [0.11.0] - 2023-01-29

### Added
- **MP4**:
  - The `InitialKey`, `ReplayGain*`, and "precise BPM" identifiers now have `ItemKey` mappings ([PR](https://github.com/Serial-ATA/lofty-rs/pull/93))
  - `AtomIdent` now implements `TryFrom<ItemKey>` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/96))
  - Added support for the vendor-supplied XID in files from the Apple iTunes store as `ItemKey::AppleXid`. ([PR](https://github.com/Serial-ATA/lofty-rs/pull/98))
  - Added support for `ItemKey::FlagCompilation` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/103))
- **Vorbis Comments**:
  - Additional mappings for the `Label`, `Remixer`, and `EncodedBy` `ItemKey` variants ([PR](https://github.com/Serial-ATA/lofty-rs/pull/94))
- **ID3v2**: A new `id3v2_compression_support` feature to optionally depend on `flate2` for decompressing frames
- **ItemKey**:
  - New Variants: `AppleXid`, `Director`, `Color`
- **AudioFile**: `AudioFile::save_to{_path}` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/104))
- **Files**: `<File>::set_{tag}`
- **FLAC**: `FlacProperties`
  - Previously, FLAC files used `FileProperties`. `FlacProperties` was added to support getting the MD5 signature
    of the audio data.
- **OGG**: `OggPictureStorage`
  - This was added to cover the overlap in functionality between `VorbisComments` and `FlacFile` in that they both
    store `(Picture, PictureInformation)`.
- **TagExt**: `TagExt::len`

### Changed
- **MP4**: `AtomIdent` stores freeform identifiers as `Cow<str>` opposed to `String` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/95))
  - This allows freeform identifiers to be constructed in a const context.
- **ID3v2**: `FrameID` now uses `Cow<str>` opposed to `String` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/102))
- **FLAC**: `FlacFile` now stores pictures separately from its `VorbisComments` tag

### Removed
- **Metadata format features** ([PR](https://github.com/Serial-ATA/lofty-rs/pull/97)):
  - All of the format-specific features have been removed, as they served no purpose. They used to bring in
    optional dependencies, but they have long since been removed.

### Fixed
- **Tag**: Handling of the `Year` tag has been improved.
  - Previously, setting a year with `Tag::set_year` required a `RecordingDate`. Now it will check if the format
  	supports the `Year` tag, and if not, then it will set a `RecordingDate`.
- **OGG**: Writing of large packets would corrupt the stream ([issue](https://github.com/Serial-ATA/lofty-rs/issues/130)) ([PR](https://github.com/Serial-ATA/lofty-rs/issues/131))

### ogg_pager
See [ogg_pager's changelog].

## [0.10.0] - 2022-12-27

### Added
- **TagExt**: `TagExt::contains`
- **Ilst**: `AtomData::Bool` for the various flag atoms such as `cpil`, `pcst`, etc.
- **BoundTaggedFile**: A `TaggedFile` variant bound to a `File` handle. ([issue](https://github.com/Serial-ATA/lofty-rs/issues/73)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/80))

### Changed
- **Files**: Return the removed tag from `<File>::remove(TagType)` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/74))
  - Previously, the only way to remove and take ownership of a tag was through `TaggedFile::take`.
    This was not possible when using a concrete type, such as `OpusFile`.
- **TaggedFile**: Renamed `TaggedFile::take` to `TaggedFile::remove` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/74))
- **lofty_attr**: The `lofty_attr::LoftyFile` derive proc macro is now exported as `lofty::LoftyFile`.
- **TaggedFile**: All methods have been split out into a new trait, `TaggedFileExt`. ([PR](https://github.com/Serial-ATA/lofty-rs/pull/80))
- **Accessor**: All methods returning string values now return `Cow<str>`. ([PR](https://github.com/Serial-ATA/lofty-rs/pull/83))
  - This is an unfortunate change that needed to be made in order to accommodate the handling of the different
    possible text separators between ID3v2 versions.
- **ID3v2**: Support reading of duplicate tags ([issue](https://github.com/Serial-ATA/lofty-rs/issues/87)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/88))
  - Previously, if we were reading a file and encountered an ID3v2 tag after having already read one,
    we would overwrite the last one, losing all of its information. Now we preserve all of the information,
    overwriting frames as necessary.

### Fixed
- **ID3v2**: The `'/'` character is no longer used as a separator ([issue](https://github.com/Serial-ATA/lofty-rs/issues/82))
- **MP4**: Stopped expecting certain flags for the `gnre` atom prior to upgrading it ([issue](https://github.com/Serial-ATA/lofty-rs/issues/84)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/85))

### ogg_pager
See [ogg_pager's changelog](ogg_pager/CHANGELOG.md).

## [0.9.0] - 2022-10-30

### Added
- `ParseOptions` ([issue](https://github.com/Serial-ATA/lofty-rs/issues/50)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/70)):
  - ⚠️ Important ⚠️: This update introduces `ParseOptions` to allow for finer grained control over error
      eagerness and other settings. Previously, when reading a file the only option available was
      `read_properties`, specified with a `bool` in `read_from{_path}`. This will now default to `true`,
      and can be overridden when using `Probe`.
- **🎉 Support for AAC (ADTS) files** ([issue](https://github.com/Serial-ATA/lofty-rs/issues/58)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/71))
- **FileProperties**: `FileProperties::new`
- Debug logging via the [log](https://crates.io/crates/log) crate for exposing recoverable errors.
- **Error**: `ErrorKind::SizeMismatch`

### Changed
- **ID3v2**:
  - Frame/tag flags with optional additional data are now `Option<T>` instead of `(bool, T)`
  - `id3::v2::TextEncoding` is now exported as `lofty::TextEncoding`
- `read_from{_path}` will no longer take a `bool` for reading properties, and will do it by default. To
  change this behavior, you must now use `Probe`.
- **FileType**: `primary_tag_type` will no longer change its return depending on the enabled features.
- **lofty_attr**: Simplified the `file_type` attribute:
  - Before, you had to specify custom file types as `#[lofty(file_type = "Custom(\"MyFile\")")]`. Now
    you can simply do `#[lofty(file_type = "MyFile")]` and it will infer the rest.
- **IFF**: `WAV` and `AIFF` items are no longer combined in the `iff` module. They are now separated
			into their own modules at `iff::wav` and `iff::aiff` respectively.

### Fixed
- **ID3v2**: Populate the `Populatimeter` field in the `ID3v2` -> `Tag` conversion ([issue](https://github.com/Serial-ATA/lofty-rs/issues/63)) ([PR](https://github.com/Serial-ATA/lofty-rs/pull/64))

### Removed
- **lofty_attr**: The `#[lofty(always_present)]` attribute has been removed, and is now inferred.

## [0.8.1] - 2022-09-09

### Added
- **VorbisComments**: `VorbisComments::get_all`, same as `Tag::get_strings` ([issue](https://github.com/Serial-ATA/lofty-rs/issues/66))

### Fixed
- **ID3v2**: Handle tag-wide unsynchronisation flag ([amberol#235](https://gitlab.gnome.org/World/amberol/-/issues/235))
- **MP3**: Stop using partial frame headers ([PR](https://github.com/Serial-ATA/lofty-rs/pull/67))

## [0.8.0] - 2022-08-10

### Added
- **A new file resolver system**:
  - New module: `lofty::resolve`
  - With this, you will be able to create your own `FileType`s, while continuing
    to use lofty's traditional API. Read the module docs for more info.
- **A proc macro for file creation**:
  - With the new `lofty_attr` crate, file creation has been simplified significantly.
    It is available for both internal and external usage.
- **ID3v2**: Exposed internal functions `id3::v2::util::{synch_u32, unsynch_u32}`
- **MP4**: `Atom::push_data`

### Changed
- **TaggedFile**: `tag{_mut}` no longer takes a reference to `TagType`
- **ID3v2**: `LanguageFrame`'s `lang` field has changed type - `String` -> `[u8; 3]`
- **MP3**:
  - Renamed `lofty::mp3` -> `lofty::mpeg`
  - Renamed `MP3File` -> `MPEGFile`
  - Renamed `MP3Properties` -> `MPEGProperties`
- **MP4**: `Atom::data` will now return all values
- **Vorbis Comments**: Recognize lowercase `METADATA_BLOCK_PICTURE` as a picture ([issue](https://github.com/Serial-ATA/lofty-rs/issues/60))

### Fixed
- **ID3v2**: `ItemKey::InitialKey` now maps to `TKEY` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/61))

## [0.7.3] - 2022-07-22

### Added
- **FileType**: `FileType::from_ext` detects MP1/MP2 as `FileType::MP3`, allowing these files to be read with
                `read_from_path`/`Probe::open`.
- **ItemKey**: `ItemKey::{REPLAYGAIN_ALBUM_GAIN, REPLAYGAIN_ALBUM_PEAK, REPLAYGAIN_TRACK_GAIN, REPLAYGAIN_TRACK_PEAK}`

### Changed
- **ID3v2**:
  - `TXXX`/`WXXX` frames will be stored by their descriptions in `ID3v2Tag` -> `Tag` conversions
  - Stopped allowing empty strings in `Accessor::set_*`

### Fixed
- **Tag**: The `Accessor::set_*` methods will stop falling through, and adding empty strings

## [0.7.2] - 2022-07-13

This release mostly addresses issues uncovered by fuzzing, thanks to [@5225225](https://github.com/5225225)!

### Changed
- **Tag**: The `Accessor::set_*` methods will now remove the item when given an empty string

### Fixed
- **AIFF/WAV**: Stop relying on the file-provided size when reading (Fixes OOM)
- **MP3/APE**: Stop trusting the lengths of APE tag items (Fixes OOM)
- **PictureInformation**: Fix potential integer overflow in `{from_jpeg, from_png}`
- **MP4**: The parser has received a major facelift, and shouldn't be so eager to allocate or trust user data (Fixes OOM)
- **FLAC**: Return early when encountering invalid zero-sized blocks
- **FLAC/Opus/Vorbis/Speex**: Add better length validity checks while reading Vorbis Comments (Fixes OOM)

## [0.7.1] - 2022-07-08

### Added
- **Vorbis Comments**: `VorbisComments::{pictures, set_picture, remove_picture}`
- **Tag**: `Tag::{set_picture, remove_picture}`
- **MP4**: Support property reading for files with FLAC audio

### Changed
- **ID3v2**: `ID3v2Tag` now derives `Eq`

## [0.7.0] - 2022-06-27

### Added
- **WavPack** support
- **Accessor**:
  - The following new accessor methods have been added:
    - `track`
    - `track_total`
    - `disk`
    - `disk_total`
    - `year`
    - `comment`

### Changed
- Bitrates in properties will be rounded up, similar to FFmpeg and TagLib
- **ID3v1**: Renamed `Id3v1Tag` -> `ID3v1Tag`
- **ID3v2**:
  - Insert multi-value frames separately when converting to `Tag`
    - E.g. An artist of "foo/bar/baz" will become 3 different `TagItem`s with `ItemKey::TrackArtist`
  - Join multiple artists with "/" during `Tag` -> `Id3v2Tag` conversion
    - Inverse of the previous entry
  - Properly capitalized the following:
    - `Id3v2Error` -> `ID3v2Error`
    - `Id3v2ErrorKind` -> `ID3v2ErrorKind`
    - `ErrorKind::Id3v2` -> `ErrorKind::ID3v2`
    - `Id3v2TagFlags` -> `ID3v2TagFlags`
    - `Id3v2Version` -> `ID3v2Version`
    - `Id3v2Tag` -> `ID3v2Tag`
- Properly capitalized the variants of `TagType`
  - `Ape` -> `APE`
  - `Id3v1` -> `ID3v1`
  - `Id3v2` -> `ID3v2`
  - `Mp4Ilst` -> `MP4ilst`
  - `RiffInfo` -> `RIFFInfo`
  - `AiffText` -> `AIFFText`
- All types implementing `PartialEq` now implement `Eq`
- **MP4**: `Ilst::track_number` has been moved to the `Accessor::track` implementation
- **Tag**: Renamed `Tag::get_texts` to `Tag::get_strings`
- **AIFF**: Renamed `AiffTextChunks` -> `AIFFTextChunks`
- **WAV**: Renamed `RiffInfoList` -> `RIFFInfoList`

### Fixed
- **AIFF**: Fixed division by zero panic during property reading ([issue](https://github.com/Serial-ATA/lofty-rs/issues/56))
- **ID3v2**: Support decoding UTF-16 T/WXXX frames with missing content BOM ([issue](https://github.com/Serial-ATA/lofty-rs/issues/53))

## [0.6.3] - 2022-05-18

### Added
- **MP4**:
  - Support atoms with multiple values ([issue](https://github.com/Serial-ATA/lofty-rs/issues/48))
  - `Atom::from_collection`

### Changed
- **ID3v2**: Discard empty frames, rather than error
- **APE**: Allow empty tag items
  - Rather than error on empty items, they will just be discarded

### Fixed
- **Pictures**: Treat "image/jpg" as `MimeType::Jpeg` ([PR](https://github.com/Serial-ATA/lofty-rs/pull/41))
- **MP3**:
  - Properly validate the contents of Xing/LAME/VBRI headers ([issue](https://github.com/Serial-ATA/lofty-rs/issues/42))
    - A header with any field zeroed out would result in a division by zero panic
  - Fix duration estimation for files with Xing headers without the necessary flags
- **FLAC**: Fix property reading of zero-length files ([issue](https://github.com/Serial-ATA/lofty-rs/issues/46))
- **Vorbis Comments**: Fix reading of vendor strings with invalid mixed UTF-8 and UTF-16 encodings
- **ID3v2**:
  - Fix reading of zero-size tags
  - Attempt to read invalid v2 frame IDs in v3 tags
    - For some reason, some apps write v2 frame IDs in otherwise valid v3 frames
  - Attempt to decode invalid `COMM` languages
- **MP4**:
  - Fix hang when reading invalid padding ([issue](https://github.com/Serial-ATA/lofty-rs/issues/44))
    - If invalid padding was encountered at the end of the file, the reader would get stuck in an infinite loop
      attempting to read zero size atoms
  - Fallback to bitrate calculation from `mdat` when necessary ([issue](https://github.com/Serial-ATA/lofty-rs/issues/43))
    - When reading a file that doesn't provide a valid bitrate or duration, a division by zero panic would occur.
      Now, it attempts to calculate the bitrate from the `mdat` atom.

## [0.6.2] - 2022-04-24

### Fixed
- **MP3**: Fix panic when reading files with no MPEG frames ([issue](https://github.com/Serial-ATA/lofty-rs/issues/39))
  - Attempting to read an MP3 file with `read_properties = true` would result in a panic if the file contained no
    MPEG frames

## [0.6.1] - 2022-04-09

### Fixed
- **MP3**: Fix reading of ID3v2 tags with an extended header
  - Restrictions were unnecessarily put on the reader, keeping it from continuing to read into the extended header
    if it was present
- **ID3v2**: Fix reading of tags with an extended header
  - The size of the extended header was not being subtracted from the total tag size, causing the reading to continue
    outside the tag boundaries

## [0.6.0] - 2022-04-05

### Added
- `TagItem::{into_key, into_value, consume}`
- **MP4**: `Mp4Codec::MP3`
- **MP4**: `mp4::AudioObjectType`
  - This new type is used in `mp4::Mp4Properties`, accessible with `Mp4Properties::audio_object_type`.
    This provides additional information for the type of audio being dealt with.
- `TagExt::clear`
  - This allows tags to be cleared of any items or pictures, while retaining any flags (if applicable)
- **ID3v2**: Respect `Id3v2TagFlags::crc` when writing
  - Previously, this flag was ignored, but it will now calculate a CRC for the extended header
- **ID3v2**: `FrameValue::Popularimeter`
- `ItemValue::{into_string, into_binary}`
- `Tag::take_strings`
- `TaggedFile` now implements `AudioFile`

### Changed
- **MP4**: Sample rates and channels are now retrieved from the [audio specific config](https://wiki.multimedia.cx/index.php?title=MPEG-4_Audio#Audio_Specific_Config) (if possible).
           If the information is invalid or unavailable, the existing value from the `mp4a` box will be used instead.
- **Vorbis Comments**: Support non-PNG/JPEG images in `PictureInformation::from_picture`
  - The method still only supports PNG and JPEG, but rather than error when it encounters an unknown image, it will return
    `PictureInformation::default`
- `lofty::read_from` will now wrap the `File` in a `BufReader`
- **FLAC**: FLAC now has its own module at `lofty::flac`
- **ID3v2**: `FrameValue` is now `#[non_exhaustive]`
- `TagType::remove_from` now works for ID3v2 tags in APE and FLAC files
  - This previously verified that the `FileType` supported the tag. It now has special exceptions for these formats to
    allow stripping out these unsupported tags
- **MP4**: Renamed `AdvisoryRating::None` to `AdvisoryRating::Inoffensive`
- Renamed `TaggedFile::remove_tag` to `TaggedFile::take`
- **Vorbis Comments**: `VorbisComments::insert_picture` now accepts a user provided `PictureInformation`
- **Vorbis Comments**: Rename `VorbisComments::{get_item, insert_item, remove_key}` to `VorbisComments::{get, insert, remove}`
- **Vorbis Comments**: `VorbisComments::remove` now returns an iterator over the removed items

### Fixed
- **MP4**: Non-full `meta` atoms are now properly handled.
  - It is possible for these to be a regular atom (no version or flags).
    This information was assumed to be present and would get skipped,
    which would affect the reading of subsequent atoms.

    This behavior has been noticed by:
    - https://leo-van-stee.github.io/
    - https://github.com/axiomatic-systems/Bento4/blob/v1.6.0-639/Source/C%2B%2B/Core/Ap4ContainerAtom.cpp#L60
    - https://github.com/taglib/taglib/issues/1041
- **MP4**: Properly search for `soun` atom
  - The search wasn't adding read bytes correctly, but tests passed due to the atom being immediately available.
    It would attempt to read until it reached an EOF if it managed to make it through multiple iterations.
- **FLAC**: Support files with an ID3v2 tag
  - This will be read only just like APE, but will allow such files to be read
- **ID3v2**: Fix writing certain proprietary Apple frames
  - When writing, frame IDs are verified with their content. The Apple specific frames "MVNM" and "MVIN" were missing,
    causing an error if they were written with their proper type (`FrameValue::Text`)
- **ID3v2**: Stop writing a BOM for `TextEncoding::UTF16BE`

## [0.5.3] - 2022-03-03

### ogg_pager
See [ogg_pager's changelog](ogg_pager/CHANGELOG.md).

## [0.5.2] - 2022-02-26

### Added
- **MP4**: `Ilst::{atoms, retain}`

### Fixed
- **ID3v2**: The footer flag is written to the tag
- **ID3v2**: Pictures are written when using `Tag`

## [0.5.1] - 2022-02-21

### Changed
- **MP4**: Padding atoms (`free`) are used when writing
- **Opus**: Channel count is verified in accordance to the channel mapping family

### Fixed
- **MP4**: `meta` atoms are written correctly

## [0.5.0] - 2022-02-20

### Added
- Support for Speex files
- `TagExt` trait to unify tag behavior
- `doc_cfg` feature for docs.rs
- Fallible allocation with `ErrorKind::Alloc` to help prevent OOM
- New dependency: `cfg-if`
- **MP3**: Emphasis struct (`mp3::Emphasis`) for use in `Mp3Properties`
- **ID3v2**: Respect the footer flag (`id3::v2::Id3v2TagFlags::footer`) when writing
- **MP4**: Constants for all well-known data types (`mp4::constants`)
- **MP4**: Support `rtng` (Parental advisory) atom, with corresponding `mp4::AdvisoryRating` enum

### Changed
- Added `#[non_exhaustive]` to `MimeType`
- Added `#[non_exhaustive]` to `PictureType`
- Added `#[non_exhaustive]` to `Mp4Codec`
- **APE**: Clarify why ID3v2 is read only
- **MP3**: No longer error on missing Xing/VBRI header when reading properties
- **MP3**: Read the entire MPEG frame header, which is exposed in `Mp3Properties`
- `AudioFile` now requires `Into<TaggedFile>`
- **MP4**: Empty atoms are discarded
- **MP4**: Variable-size integers are shrunk when writing

### Fixed
- **MP4**: Panic in `Mp4File::read_from` ([commit](https://github.com/Serial-ATA/lofty-rs/commit/9e18616a6882c659ba2d5ca6bdad9bf41171135d))
- **WAV/AIFF**: Chunk reading now makes use of fallible allocation, preventing OOM
- **ID3v2**: Text is properly encoded when writing
- **ID3v2**: `MVNM` and `MVIN` frames are now treated as text frames
- **ID3v2**: Text encodings are verified for V2 tags
- **MP4**: `plID` atom is properly treated as a 64-bit signed integer ([issue](https://github.com/Serial-ATA/lofty-rs/issues/34))
- **MP4**: `rate` and `rtng` now map to the correct `ItemKey`
- **MP4**: Integer pairs are now written correctly
- `TagType` and `FileType` are no longer taken by reference in any method

### Removed
- `ErrorKind::BadExtension`

[Unreleased]: https://github.com/Serial-ATA/lofty-rs/compare/0.22.4...HEAD
[0.22.4]: https://github.com/Serial-ATA/lofty-rs/compare/0.22.3...0.22.4
[0.22.3]: https://github.com/Serial-ATA/lofty-rs/compare/0.22.2...0.22.3
[0.22.2]: https://github.com/Serial-ATA/lofty-rs/compare/0.22.1...0.22.2
[0.22.1]: https://github.com/Serial-ATA/lofty-rs/compare/0.22.0...0.22.1
[0.22.0]: https://github.com/Serial-ATA/lofty-rs/compare/0.21.1...0.22.0
[0.21.1]: https://github.com/Serial-ATA/lofty-rs/compare/0.21.0...0.21.1
[0.21.0]: https://github.com/Serial-ATA/lofty-rs/compare/0.20.1...0.21.0
[0.20.1]: https://github.com/Serial-ATA/lofty-rs/compare/0.20.0...0.20.1
[0.20.0]: https://github.com/Serial-ATA/lofty-rs/compare/0.19.2...0.20.0
[0.19.2]: https://github.com/Serial-ATA/lofty-rs/compare/0.19.1...0.19.2
[0.19.1]: https://github.com/Serial-ATA/lofty-rs/compare/0.19.0...0.19.1
[0.19.0]: https://github.com/Serial-ATA/lofty-rs/compare/0.18.2...0.19.0
[0.18.2]: https://github.com/Serial-ATA/lofty-rs/compare/0.18.1...0.18.2
[0.18.1]: https://github.com/Serial-ATA/lofty-rs/compare/0.18.0...0.18.1
[0.18.0]: https://github.com/Serial-ATA/lofty-rs/compare/0.17.1...0.18.0
[0.17.1]: https://github.com/Serial-ATA/lofty-rs/compare/0.17.0...0.17.1
[0.17.0]: https://github.com/Serial-ATA/lofty-rs/compare/0.16.1...0.17.0
[0.16.1]: https://github.com/Serial-ATA/lofty-rs/compare/0.16.0...0.16.1
[0.16.0]: https://github.com/Serial-ATA/lofty-rs/compare/0.15.0...0.16.0
[0.15.0]: https://github.com/Serial-ATA/lofty-rs/compare/0.14.0...0.15.0
[0.14.0]: https://github.com/Serial-ATA/lofty-rs/compare/0.13.0...0.14.0
[0.13.0]: https://github.com/Serial-ATA/lofty-rs/compare/0.12.1...0.13.0
[0.12.1]: https://github.com/Serial-ATA/lofty-rs/compare/0.12.0...0.12.1
[0.12.0]: https://github.com/Serial-ATA/lofty-rs/compare/0.11.0...0.12.0
[0.11.0]: https://github.com/Serial-ATA/lofty-rs/compare/0.10.0...0.11.0
[0.10.0]: https://github.com/Serial-ATA/lofty-rs/compare/0.9.0...0.10.0
[0.9.0]: https://github.com/Serial-ATA/lofty-rs/compare/0.8.1...0.9.0
[0.8.1]: https://github.com/Serial-ATA/lofty-rs/compare/0.8.0...0.8.1
[0.8.0]: https://github.com/Serial-ATA/lofty-rs/compare/0.7.3...0.8.0
[0.7.3]: https://github.com/Serial-ATA/lofty-rs/compare/0.7.2...0.7.3
[0.7.2]: https://github.com/Serial-ATA/lofty-rs/compare/0.7.1...0.7.2
[0.7.1]: https://github.com/Serial-ATA/lofty-rs/compare/0.7.0...0.7.1
[0.7.0]: https://github.com/Serial-ATA/lofty-rs/compare/0.6.3...0.7.0
[0.6.3]: https://github.com/Serial-ATA/lofty-rs/compare/0.6.2...0.6.3
[0.6.2]: https://github.com/Serial-ATA/lofty-rs/compare/0.6.1...0.6.2
[0.6.1]: https://github.com/Serial-ATA/lofty-rs/compare/0.6.0...0.6.1
[0.6.0]: https://github.com/Serial-ATA/lofty-rs/compare/0.5.3...0.6.0
[0.5.3]: https://github.com/Serial-ATA/lofty-rs/compare/0.5.2...0.5.3
[0.5.2]: https://github.com/Serial-ATA/lofty-rs/compare/0.5.1...0.5.2
[0.5.1]: https://github.com/Serial-ATA/lofty-rs/compare/0.5.0...0.5.1
[0.5.0]: https://github.com/Serial-ATA/lofty-rs/compare/64f0eff...0.5.0

[serde]: https://docs.rs/serde
[TagLib]: https://github.com/taglib/taglib
[AcoustID]: https://acoustid.org/
[ogg_pager's changelog]: ogg_pager/CHANGELOG.md
