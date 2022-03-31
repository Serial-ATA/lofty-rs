# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

## [0.5.3] - 2022-03-03

### Fixed
- **OGG**: Segment tables are written correctly with data spanning multiple pages ([issue](https://github.com/Serial-ATA/lofty-rs/issues/37))

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

[Unreleased]: https://github.com/Serial-ATA/lofty-rs/compare/74d9f35...main
[0.5.3]: https://github.com/Serial-ATA/lofty-rs/compare/5bf1f34...74d9f35
[0.5.2]: https://github.com/Serial-ATA/lofty-rs/compare/d00be2c...5bf1f34
[0.5.1]: https://github.com/Serial-ATA/lofty-rs/compare/a1463f3...d00be2c
[0.5.0]: https://github.com/Serial-ATA/lofty-rs/compare/64f0eff...a1463f3