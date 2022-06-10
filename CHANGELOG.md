# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **WavPack** support

### Changed
- Bitrates in properties will be rounded up, similar to FFmpeg and TagLib
- **ID3v2**: Insert multi-value frames separately when converting to `Tag`
  - E.g. An artist of "foo/bar/baz" will become 3 different `TagItem`s with `ItemKey::TrackArtist`
- Properly capitalized the variants of `TagType`
  - `Ape` -> `APE`
  - `Id3v1` -> `ID3v1`
  - `Id3v2` -> `ID3v2`
  - `Mp4Ilst` -> `MP4ilst`
  - `RiffInfo` -> `RIFFInfo`
  - `AiffText` -> `AIFFText`
- All types implementing `PartialEq` now implement `Eq`

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

[Unreleased]: https://github.com/Serial-ATA/lofty-rs/compare/3065bdb...main
[0.6.3]: https://github.com/Serial-ATA/lofty-rs/compare/868d6b4...3065bdb
[0.6.2]: https://github.com/Serial-ATA/lofty-rs/compare/87faae7...868d6b4
[0.6.1]: https://github.com/Serial-ATA/lofty-rs/compare/f1f2a5c...87faae7
[0.6.0]: https://github.com/Serial-ATA/lofty-rs/compare/74d9f35...f1f2a5c
[0.5.3]: https://github.com/Serial-ATA/lofty-rs/compare/5bf1f34...74d9f35
[0.5.2]: https://github.com/Serial-ATA/lofty-rs/compare/d00be2c...5bf1f34
[0.5.1]: https://github.com/Serial-ATA/lofty-rs/compare/a1463f3...d00be2c
[0.5.0]: https://github.com/Serial-ATA/lofty-rs/compare/64f0eff...a1463f3
