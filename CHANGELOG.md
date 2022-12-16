# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **TagExt**: `TagExt::contains`
- **Ilst**: `AtomData::Bool` for the various flag atoms such as `cpil`, `pcst`, etc.
- **ogg_pager**: Support for reading packets with the new `Packets` struct.
- **ogg_pager**: `PageHeader` struct.
- **BoundTaggedFile**: A `TaggedFile` variant bound to a `File` handle.

### Changed
- **Files**: Return the removed tag from `<File>::remove(TagType)`
  - Previously, the only way to remove and take ownership of a tag was through `TaggedFile::take`.
    This was not possible when using a concrete type, such as `OpusFile`.
- **TaggedFile**: Renamed `TaggedFile::take` to `TaggedFile::remove`
- **OGG**: The reading of OGG files has switched to using packets opposed to pages, making it more
           spec-compliant and efficient.
- **ogg_pager**: Most fields in `Page` have been separated out into the new `PageHeader` struct.
- **ogg_pager**: `paginate` now works with a collection of packets.
- **lofty_attr**: The `lofty_attr::LoftyFile` derive proc macro is now exported as `lofty::LoftyFile`.
- **TaggedFile**: All methods have been split out into a new trait, `TaggedFileExt`.
- **Accessor**: All methods returning string values now return `Cow<str>`.
  - This is an unfortunate change that needed to be made in order to accommodate the handling of the different
    possible text separators between ID3v2 versions.

### Removed
- **ogg_pager**: Removed `Page::new`, now pages can only be created through `ogg_pager::paginate` or
                 `Packets::paginate`.

### Fixed
- **ID3v2**: The `'/'` character is no longer used as a separator ([issue](https://github.com/Serial-ATA/lofty-rs/issues/82))
- **MP4**: Empty atoms were able to pass through and get stored, causing a panic in the `Ilst` -> `Tag` conversion ([issue](https://github.com/Serial-ATA/lofty-rs/issues/84))

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

[Unreleased]: https://github.com/Serial-ATA/lofty-rs/compare/0.9.0...HEAD
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
