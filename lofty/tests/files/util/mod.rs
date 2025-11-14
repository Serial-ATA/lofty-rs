use lofty::config::{ParseOptions, WriteOptions};
use lofty::error::LoftyError;
use lofty::file::{AudioFile, BoundTaggedFile, TaggedFileExt};
use lofty::io::{FileLike, Length, Truncate};
use lofty::probe::Probe;
use lofty::tag::{ItemKey, TagExt, TagType};
use std::fs::File;
use std::io::{Seek as _, Write as _};
use std::path::Path;

/// Create a new temporary file and copy the contents of `path` into it
pub fn temp_file(path: impl AsRef<Path>) -> File {
	let content = std::fs::read(path).unwrap();

	let mut file = tempfile::tempfile().unwrap();
	file.write_all(&content).unwrap();
	file.rewind().unwrap();

	file
}

/// Copy `path` into a [`temp_file()`] and parse it via [`Probe`]
pub fn read(path: impl AsRef<Path>) -> BoundTaggedFile<File> {
	let file = temp_file(path);

	Probe::new(file)
		.options(ParseOptions::new())
		.guess_file_type()
		.unwrap()
		.read_bound()
		.unwrap()
}

/// Verify that the file at `path` has no tags
///
/// Some formats (like Opus) *require* a tag, so `expected_len` can be used to check that the tag has
/// the bare minimum number of values.
pub fn no_tag_test(path: impl AsRef<Path>, expected_len: Option<usize>) {
	let mut file = temp_file(path);
	let tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_tags(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();

	let Some(expected_len) = expected_len else {
		assert!(!tagged_file.contains_tag());
		return;
	};

	for tag in tagged_file.tags() {
		assert_eq!(tag.len(), expected_len);
	}
}

/// Verify that no audio properties are read when requested
pub fn no_properties_test(path: impl AsRef<Path>) {
	let mut file = temp_file(path);
	let tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();
	assert!(tagged_file.properties().is_empty());
}

/// Verify that the tag of type `tag_type` has an [`ItemKey::TrackArtist`] of `expected_value`
///
/// Also verifies that the tag has exactly `expected_item_count` items
pub fn verify_artist(
	file: &impl TaggedFileExt,
	tag_type: TagType,
	expected_value: &str,
	expected_item_count: u32,
) {
	println!(
		"VERIFY: Expecting `{tag_type:?}` to have {expected_item_count} items, with an artist of \
		 \"{expected_value}\""
	);

	assert!(file.tag(tag_type).is_some());

	let tag = file.tag(tag_type).unwrap();

	assert_eq!(tag.item_count(), expected_item_count);

	assert_eq!(
		tag.get(ItemKey::TrackArtist),
		Some(&lofty::tag::TagItem::new(
			ItemKey::TrackArtist,
			lofty::tag::ItemValue::Text(String::from(expected_value))
		))
	);
}

/// This will:
///
/// * Verify that the tag of type `tag_type` has an artist of `expected_value`
/// * Set the artist to `new_value`
/// * Write the tag back to the file
pub fn set_artist<F: FileLike>(
	tagged_file: &mut BoundTaggedFile<F>,
	tag_type: TagType,
	expected_value: &str,
	new_value: &str,
	expected_item_count: u32,
) where
	LoftyError: From<<F as Truncate>::Error>,
	LoftyError: From<<F as Length>::Error>,
{
	verify_artist(tagged_file, tag_type, expected_value, expected_item_count);
	println!("WRITE: Writing artist \"{new_value}\" to {tag_type:?}\n",);

	let tag = tagged_file.tag_mut(tag_type).unwrap();

	tag.insert_unchecked(lofty::tag::TagItem::new(
		ItemKey::TrackArtist,
		lofty::tag::ItemValue::Text(String::from(new_value)),
	));

	tagged_file.save(WriteOptions::default()).unwrap();
}

/// Test tag removal
///
/// 1. Reads the file at `path`
/// 2. Removes the tag of type `tag_type`
/// 3. Re-reads, verifying the tag is removed
pub fn remove_tag_test(path: impl AsRef<Path>, tag_type: TagType) {
	let mut file = temp_file(path);

	let tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();
	assert!(tagged_file.tag(tag_type).is_some());

	file.rewind().unwrap();

	tag_type.remove_from(&mut file).unwrap();

	file.rewind().unwrap();

	let tagged_file = Probe::new(&mut file)
		.options(ParseOptions::new().read_properties(false))
		.guess_file_type()
		.unwrap()
		.read()
		.unwrap();
	assert!(tagged_file.tag(tag_type).is_none());
}
