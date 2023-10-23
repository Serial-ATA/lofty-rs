# Benchmarks

There are two categories of benchmarks here:
* File parsing for each of the supported file formats
* Tag creation for each of the supported tag formats

## File parsing

The file parsing benchmarks are run on a set of assets that attempt to be representative of
files one would find in the wild.

The song used is "TempleOS Hymn Risen (Remix)" by Dave Eddy, licensed under Public Domain.

Links to the song:
* [YouTube](https://www.youtube.com/watch?v=IdYMA6hY_74)
* [Bandcamp](https://daveeddy.bandcamp.com/track/templeos-hymn-risen-remix)
* [Creator's site](https://music.daveeddy.com/tracks/templeos-hymn-risen-remix/)

The file was originally provided as a FLAC, and has been re-encoded to the other formats.

Some conditions:

* Each file will only make use of their ["primary tag"](https://docs.rs/lofty/latest/lofty/enum.FileType.html#method.primary_tag_type).
* The following fields are used (with some possibly left out, depending on the format):
  * Title
  * Artist
  * Album
  * Date
  * Track number
  * Genre
  * Picture (Front cover)
  * Encoder

### Tag creation

The tag creation benchmarks will only create the tags and dump them to a writer,
this will not take into account the time it takes to write the tags to a file.

The tags will be created using the same conditions as above, with the exact same data as present in the files.
