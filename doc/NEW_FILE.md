# Adding a new file format

## Table of Contents

1. [Intro](#intro)
2. [Directory Layout](#directory-layout)
3. [Defining the File](#defining-the-file)
    * [Adding the FileType](#adding-the-filetype)
    * [The File Struct](#the-file-struct)
    * [Reading the File](#reading-the-file)
    * [Tests](#tests)
        * [Unit Tests](#unit-tests)
        * [Integration Tests](#integration-tests)
        * [Fuzz Tests](#fuzz-tests)

## Intro

**Note that while this is a simple example, there have been more complex definitions. Be sure to check the implementations
of existing file types.**

This document will cover the implementation of an audio file format named "Foo".

* This format supports the following tag formats: ID3v2 and APE.
* Has the extension `.foo`
* Has the magic signature `0x0F00`

## Directory Layout

To define a new file format, create a new directory under `src/`. In this case, it will be
`src/foo`.

There are some files that every file format needs:

* `mod.rs` - Stores the file struct definition and any module exports
* `read.rs` - Handles reading the format for tags and other relevant information
* `properties.rs` - Handles reading the properties from the file, likely from a fragment read
                    from `read.rs`.

Now, the directory should look like this:

```
src/
└── foo/
    ├── mod.rs
    ├── read.rs
    └── properties.rs
```

Note that in the case that a format has its own tagging format, similar to how the APE format uses APE tags,
you would define that tag in a subdirectory of `src/foo`. See [NEW_TAG.md](NEW_TAG.md).

## Defining the File

Now that the directories are created, we can start working on defining our file.

### Adding the FileType

Before we can define the file struct, we need to add a variant to `FileType`.

Go to [src/file.rs](../src/file.rs) and edit the `FileType` enum to add your new variant.

```rust
pub enum FileType {
    Foo,
    // ...
}
```

Now, we will have to specify the primary tag type in `FileType::primary_tag_type()`.
Let's say that the Foo format primarily uses ID3v2:

```rust
pub fn primary_tag_type(&self) -> TagType {
    match self {
        FileType::Aiff | FileType::Mpeg | FileType::Wav | FileType::Aac | FileType::Foo => TagType::Id3v2,
        // ...
    }
}
```

Finally, we need to specify the extension(s) and magic signature of the format.

Firstly, the extension is defined in `FileType::from_ext()`:

```rust
pub fn from_ext<E>(ext: E) -> Option<Self>
where
	E: AsRef<OsStr>,
{
	let ext = ext.as_ref().to_str()?.to_ascii_lowercase();

	match ext.as_str() {
		"foo" => Some(Self::Foo),
		// ...
	}
}
```

Then we can check the magic signature in `FileType::quick_type_guess()`:

```rust
fn quick_type_guess(buf: &[u8]) -> Option<Self> {
    use crate::mpeg::header::verify_frame_sync;

    // Safe to index, since we return early on an empty buffer
    match buf[0] {
        0x0F if buf.starts_with(0x0F00) => Some(Self::Foo),
        // ...
    }
}
```

Now that we have the `FileType` variant fully specified, we need to add it to `lofty_attr`.

Go to [lofty_attr/src/internal.rs](../lofty_attr/src/internal.rs) and add the variant to `LOFTY_FILE_TYPES`.

```rust
const LOFTY_FILE_TYPES: [&str; N] = [
	"Foo", // ...
];
```

### The File Struct

Now we can define our file struct in `src/foo/mod.rs`.

Unless there is additional information to provide from the format, such as [`Mp4File::ftyp()`](https://docs.rs/lofty/latest/lofty/mp4/struct.Mp4File.html#method.ftyp),
this file can simply be a struct definition (with exports as necessary).

```rust
mod read;
mod properties;

use crate::ape::tag::ApeTag;
use crate::id3::v2::tag::Id3v2Tag;
use crate::properties::FileProperties;

// This does most of the work
use lofty_attr::LoftyFile;

/// A Foo file
#[derive(LoftyFile)]
#[lofty(read_fn = "read::read_from")]
pub struct FooFile {
    #[lofty(tag_type = "Id3v2")]
    pub(crate) id3v2_tag: Option<Id3v2Tag>,
    #[lofty(tag_type = "Ape")]
    pub(crate) ape_tag: Option<ApeTag>,
    pub(crate) properties: FileProperties,
}
```

And the file is now defined!

This is essentially the same as the [custom resolver example](https://github.com/Serial-ATA/lofty-rs/blob/main/examples/custom_resolver/src/main.rs),
except we do not need to go through the `FileResolver` API nor specify a `FileType` (this is handled by `lofty_attr`).

### Reading the File

The file reading is handled in `read.rs`, housing a function with the following signature:

```rust
use super::FooFile;
use crate::error::Result;
use crate::probe::ParseOptions;

use std::io::{Read, Seek};

pub(super) fn read_from<R>(reader: &mut R, parse_options: ParseOptions) -> Result<FooFile>
where
    R: Read + Seek;
```

Some notes on file parsing:

* You will need to verify the file's magic signature again
* You should only gather the information necessary for property reading (such as additional chunks) only if 
  `parse_options.read_properties` is true.
* There should be no handling of properties here, that is saved for `properties.rs`
* There are many utilities to easily find and parse tags, such as `crate::id3::{find_id3v2, find_id3v1, find_lyrics3v2}, crate::ape::tag::read::{read_ape_tag}, etc.`
* And most importantly, look at existing implementations! There is a high chance that what is being attempted has already been done
  in some capacity.

### Tests

#### Unit Tests

The only mandatory unit tests are for property reading. These are stored in `src/properties.rs`.

#### Integration Tests

Before creating integration tests, make a version of your file that has all possible tags in it. For example, a Foo file with an ID3v2 tag and an APE tag.

Put this file in `tests/files/assets/minimal/full_test.{ext}`

Then we'll store our tests in `tests/files/{format}.rs`.

There is a simple suite of tests to go in that file:

* `read()`: Read a file containing all possible tags (a Foo file with an ID3v2 *and* APE tag in this case), verifying
  all expected information is present. This can be done quickly with the `crate::verify_artist()` function.
* `write()`: Change the artist field of each tag using the `crate::set_artist()` function, which will verify the artist
  and set a new one. Then, revert the artists using the same method.
* `remove_{tag}()`: For each tag format the file supports, create a `remove_{tag}()` test that simply calls the
  `crate::remove_tag_test()` function. For example, this format would have `remove_ape()` and `remove_id3v2()`.

#### Fuzz Tests

Fuzz targets are stored in `fuzz/fuzz_targets/{format}file_read_from`

They can be easily defined in a few lines:

```rust
#![no_main]

use std::io::Cursor;

use libfuzzer_sys::fuzz_target;
use lofty::{AudioFile, ParseOptions};

fuzz_target!(|data: Vec<u8>| {
    let _ = lofty::foo::FooFile::read_from(
	    &mut Cursor::new(data),
	    ParseOptions::new().read_properties(false),
    );
});
```


