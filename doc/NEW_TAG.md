# Adding a new tag format

## Table of Contents

1. [Intro](#intro)
2. [Directory Layout](#directory-layout)
3. [Defining the Tag](#defining-the-tag)
	* [Adding the TagType](#adding-the-tagtype)
	* [The Tag Struct](#the-tag-struct)
	* [Implementing TagExt](#implementing-tagext)
      * [Converting Into Tag](#converting-into-tag)
        * [Defining Generic Mappings](#defining-generic-mappings)
        * [Split and Merge Tag](#split-and-merge-tag)
4. [Writing](#writing)
5. [Tests](#tests)
    * [Assets](#assets)
    * [Unit Tests](#unit-tests)
    * [Integration Tests](#integration-tests)
    * [Fuzz Tests](#fuzz-tests)

## Intro

**Note that while this is a simple example, there have been more complex definitions. Be sure to check the implementations
of existing tag types.**

This document will cover the implementation of a tag file format named "Foo".

* It is a simple UTF-8 key/value storage
* The layout is `xxxxYYYY\0ZZZZ`, with `x` being the size of the item, `Y` being the key, and `Z` being the value
* It has support for track title, artist, and album name
* It is supported in the Foo audio format we created in [doc/NEW_FILE.md](../doc/NEW_FILE.md)

## Directory Layout

To define a new tag format, first determine if it is supported by a single format. In this example it is,
so we would place its definition in a subdirectory of the `foo` directory, where we defined the Foo audio format.
If this was a generic tag format supported by multiple audio formats, like ID3, you'd simply define it in its own
folder inside [src/](../src).

There are some files that every tag needs:

* `mod.rs` - Stores the tag struct definition and any module exports
* `read.rs` - Handles reading the tag
* `write.rs` - Handles writing the tag to any format that supports it

Now, the directory should look like this:

```
src/
└── foo/
    └── tag/
        ├── mod.rs
        ├── read.rs
        └── write.rs
```

## Defining the Tag

Now that the directories are created, we can start working on defining our file.

### Adding the TagType

Before we can define the tag struct, we need to add a variant to `TagType`.

Go to [src/tag/mod.rs](../src/tag/mod.rs) and edit the `TagType` enum to add your new variant.

```rust
pub enum TagType {
    Foo,
    // ...
}
```

### The Tag Struct

Now we can define our file struct in `src/foo/tag/mod.rs`.

```rust
mod read;
mod write;

pub struct FooTag {}
```

The internal structure of the tag does not matter much, so in this case we can just make it a
`Vec<(String, String)>`.

```rust
pub struct FooTag {
    items: Vec<(String, String)>
}
```

Now, we need to specify which `FileType`s this tag supports. For this, we use the `tag` attribute macro
from `lofty_attr`, which will generate a `FooTag::SUPPORTED_FORMATS` as well as a `FooTag::READ_ONLY_FORMATS` if we
specify any read only formats. Additionally, it will generate doc comments to make this information user-facing.

```rust
// This does most of the work
use lofty_attr::tag;

// We specify a description and supported formats
#[tag(description = "A Foo tag", supported_formats(Foo))]
pub struct FooTag {
	items: Vec<(String, String)>
}
```

If your tag happens to require read-only support for certain formats, the `FileType`s can easily be specified within the
`supported_formats` like so:

```rust
// Now we state we support *reading* the tag in MPEG files, but we will only allow the
// user to **remove** the tag, not write a new one.
#[tag(description = "A Foo tag", supported_formats(Foo, read_only(Mpeg)))]
pub struct FooTag {
	items: Vec<(String, String)>
}
```

And the tag is now defined!

Now we can move on to....

### Implementing TagExt

The primary interface for tags is through `TagExt` which, in addition to its own methods, requires an implementation
of `Accessor` and `Into<Tag>`. The latter will be discussed later. For now, we will focus on `Accessor`.

The following should work on its own:

```rust
impl Accessor for FooTag {}
```

However, every method will now return `None`.

As each tag format has its own supported set of items, we cannot guarantee that any one will be available.
So, with `Accessor` one must specify the methods they wish to implement.

Remember above we specified this format to only support the track title, artist, and album. We will now implement the
setters and getters for those items.

```rust
impl Accessor for FooTag {
    fn title(&self) -> Option<Cow<'_, str>> { /**/ }
    fn set_title(&mut self, value: String) { /**/ }
    fn remove_title(&mut self) { /**/ }

    fn artist(&self) -> Option<Cow<'_, str>> { /**/ }
    fn set_artist(&mut self, value: String) { /**/ }
    fn remove_artist() { /**/ }

    fn album(&self) -> Option<Cow<'_, str>> { /**/ }
    fn set_album(&mut self, value: String) { /**/ }
    fn remove_album(&mut self) { /**/ }
}
```

With `Accessor` being relatively simple, you will oftentimes find that the tag format ends up supporting every
method available. Typically, when that occurs, a macro named `impl_accessor` can be created at the top of the file
to prevent repetition. See the [VorbisComments](https://github.com/Serial-ATA/lofty-rs/blob/bdfe1a8cfc0648f647c625d2afb95c9a50eee81d/src/ogg/tag.rs#L20-L38)
definition for an example.

Now, in order to actually implement our `Accessor` methods, we'll need to create a getter, setter, and remover on
the tag itself.

**Note that some formats allow keys to appear multiple times, in which case you should create two separate methods,
one called `insert` and the other called `push`. `insert` will remove all other occurrences of the key and then store it,
while `push` will simply append it to the list.**

For simplicity, we will only have an `insert` method.

With our tag being a simple key/value mapping we can just iterate our keys until we find the correct value.

```rust
impl FooTag {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.items
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }

    pub fn insert(&mut self, key: String, value: String) {
        self.items.retain(|(k, _)| !k.eq_ignore_ascii_case(&key));
        self.items.push((key, value))
    }

    pub fn remove<'a>(&'a mut self, key: &str) -> impl Iterator<Item=String> + use<'a> {
        self.items.retain(|(k, _)| !k.eq_ignore_ascii_case(&key));
    }
}
```

Now the `Accessor` implementation can be finished:

```rust
impl Accessor for FooTag {
    fn title(&self) -> Option<Cow<'_, str>> {
        self.get("TITLE")
    }
    fn set_title(&mut self, value: String) {
        self.insert(String::from("TITLE"), value)
    }
    fn remove_title(&mut self) {
        self.remove("TITLE")
    }

    fn artist(&self) -> Option<Cow<'_, str>> {
        self.get("ARTIST")
    }
    fn set_artist(&mut self, value: String) {
        self.insert(String::from("ARTIST"), value)
    }
    fn remove_artist(&mut self) {
        self.remove("ARTIST")
    }

    fn album(&self) -> Option<Cow<'_, str>> {
        self.get("ALBUM")
    }
    fn set_album(&mut self, value: String) {
        self.insert(String::from("ALBUM"), value)
    }
    fn remove_album(&mut self) {
        self.remove("ALBUM")
    }
}
```

#### Converting Into Tag

The next part of `TagExt` can potentially be quite complicated/expensive depending on the tag format.

Converting your concrete tag type into the generic `Tag` involves the following:

* Defining the mappings from the concrete format's keys into the generic `ItemKey`
* Implementing `SplitTag` and `MergeTag`

##### Defining Generic Mappings

The `ItemKey` mappings are defined in [src/tag/item.rs](../src/tag/item.rs).

See the comments for the `gen_map!` macro, which explains its use in detail, and will be kept up to
date with any future changes.

We will be using the `gen_map!` macro to define the mapping between our 3 keys:

```rust
gen_map!(
    FOO_MAP;

    "TITLE" => TrackTitle,
    "ARIST" => TrackArtist,
    "ALBUM" => AlbumTitle,
);
```

Afterwards, we just need to add it into the list of maps in the `gen_item_keys!` macro:

```rust
gen_item_keys!(
    MAPS => [
        // ...

        [TagType::Foo, FOO_MAP]
    ];

    // ...
);
```

##### Split and Merge Tag

We now need to define a way for the user to split the concrete tag into its generic counterpart, and merge
it back in at will. This is done with the `SplitTag` and `MergeTag` traits.

We'll first cover `SplitTag`

The `SplitTag` trait provides a way to take every item that can be expressed in a generic way (think artist, title, etc.)
and put them into a `Tag`. The remaining items that cannot easily be expressed in `Tag` will remain in the original
tag, in an immutable wrapper.

Implementing `SplitTag` in the case of `FooTag` will be quite simple, but this can easily become very complicated.

The trait provides one method, so lets implement it:

```rust
impl SplitTag for FooTag {
    type Remainder = /* ? */;

    fn split_tag(mut self) -> (Self::Remainder, Tag) {
        todo!()
    }
}
```

You'll notice that we need to provide a `Remainder` type. This is the immutable wrapper that was mentioned earlier.
Creating this wrapper is as simple as creating a tuple struct named `SplitTagRemainder` which we can convert back
into the concrete tag, or use to get immutable access to the tag.

```rust
#[derive(Debug, Clone, Default)]
pub struct SplitTagRemainder(FooTag);

impl From<SplitTagRemainder> for FooTag {
    fn from(from: SplitTagRemainder) -> Self {
        from.0
    }
}

impl Deref for SplitTagRemainder {
    type Target = FooTag;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
```

That is all that we need for `SplitTagRemainder`. This template can be used for any tag by simply switching out `FooTag`.

Now, we can actually implement the `split_tag` method.

Since we were able to [map every possible key to an ItemKey](#defining-generic-mappings), we can simply iterate over each
item and use `ItemKey::from_key` to convert our string keys to `ItemKey`s. The inverse method, `ItemKey::map_key` will be used
later in `MergeTag`, making its implementation just as simple.

```rust
impl SplitTag for FooTag {
    type Remainder = SplitTagRemainder;

    fn split_tag(mut self) -> (Self::Remainder, Tag) {
        let mut tag = Tag::new(TagType::Foo);

        for (k, v) in std::mem::take(&mut self.items) {
            tag.items.push(TagItem::new(
                ItemKey::from_key(TagType::Foo, &k),
                ItemValue::Text(v)
            ));
        }

        // In the case of this format, the remainder will always be empty.
        // This will almost never be the case for a real-world tag format, though!
        (SplitTagRemainder(self), tag)
    }
}
```

Now callers can split their `FooTag` into a generic `Tag`, but we'll need a way to merge them back together.
This is done with the `MergeTag` trait.

When implementing `MergeTag`, one may need to take the distinction between `ItemValue::Text` and `ItemValue::Locator` into consideration.
  * In ID3v2 for example, a locator is only valid for frames starting with W.
  * In a format such as VorbisComments, there is no need to distinguish between the two.

With that in mind, we'll now implement `MergeTag` on `SplitTagRemainder`:

```rust
impl MergeTag for SplitTagRemainder {
    type Merged = FooTag;

    fn merge_tag(self, mut tag: Tag) -> Self::Merged {
        let Self(mut merged) = self;

        for item in tag.items {
            let item_key = item.item_key;
            let item_value = item.item_value;

            // We are a text only format
            let ItemValue::Text(val) = item_value else {
                continue
            };

            // We do not support unknown keys
            let Some(key) = item_key.map_key(TagType::Foo, false) else {
                continue
            };

            merged.items.push((key.to_string(), val));
        }

        merged
    }
}
```

With these two traits implemented, all that's left is to implement `From<FooTag> for Tag` and `From<Tag> for FooTag`.
`SplitTag` and `MergeTag` give us these essentially for free:

```rust
impl From<FooTag> for Tag {
    fn from(input: FooTag) -> Self {
        input.split_tag().1
    }
}

impl From<Tag> for FooTag {
    fn from(input: Tag) -> Self {
        SplitTagRemainder::default().merge_tag(input)
    }
}
```

### Writing

TODO

### Tests

#### Assets

Test assets for tag formats are to be placed in [tests/tags/assets/](../tests/tags/assets).

There should at least be one asset, which is a binary file containing the tag below:

* Title: "Foo title"
* Artist: "Bar artist"
* Album: "Baz album"
* Comment: "Qux comment"
* Year: 1984
* Track Number: 1
* Genre: "Classical"

Any of these fields can be omitted if the format does not support it.

To create this file, you can simply add the tags to a file in a program such as [Kid3](https://kid3.kde.org/).
You can then use your favorite hex editor to extract the tag, and paste them into a file in the assets directory.

The file should be named `test.{ext}`, where `ext` is the tag name. So in this example, it would be `test.foo`.

#### Unit Tests

There are at least 4 unit tests that should be created for every tag format:

* `parse_{tag}` - Tests reading the asset created above
  * Simply reads the asset and compares it to a manually constructed tag with the same data
* `{tag}_re_read` - Tests reading the asset, writing it back, and reading it again
  * Read the tag, dump it with `TagExt::dump_to()`, and reread it
* `{tag}_to_tag` - Tests converting the tag into a generic `Tag`
  * Using `crate::tag::utils::test_utils::verify_tag()`, verify that the tag is correct
* `tag_to_{tag}` - Tests converting the generic `Tag` into the concrete tag
  * Using `crate::tag::utils::test_utils::create_tag()`, verify that the converted tag is correct

These tests should be placed in the tag's `read` module. If there are many tests, feel free to break them out
into their own module (ex. See the [ID3v2 `tests` module](../src/id3/v2/tag)).

For an example of these tests, see the [ApeTag tests](https://github.com/Serial-ATA/lofty-rs/blob/9c0ea926c690bc6338ba95aceccc4d93e2ee9826/src/ape/tag/mod.rs#L540-L656).

#### Integration Tests

Integration testing is not normally necessary for tag formats, as they are typically
tested extensively through the module's unit tests. However, if one wants to create integration tests,
they can be placed in [tests/tags/](../tests/tags).
