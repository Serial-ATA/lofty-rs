use lofty::{FileType, ItemKey, ItemValue, Probe, Tag, TagItem};

#[test]
fn flac_read() {
	let file = Probe::new().read_from_path("tests/assets/a.flac").unwrap();

	assert_eq!(file.file_type(), &FileType::FLAC);

	// FLAC does **not** require a Vorbis comment block be present, this file has one
	assert!(file.first_tag().is_some());

	// primary and first tag are identical for FLAC
	let tag = file.primary_tag().unwrap();

	// We have the vendor string and a title stored in the tag
	assert_eq!(tag.item_count(), 2);

	assert_eq!(
		tag.get_item_ref(&ItemKey::TrackTitle),
		Some(&TagItem::new(
			ItemKey::TrackTitle,
			ItemValue::Text(String::from("Foo title"))
		))
	);
}

#[test]
fn flac_write() {
	let mut file = std::fs::OpenOptions::new()
		.read(true)
		.write(true)
		.open("tests/assets/a.flac")
		.unwrap();

	let mut tagged_file = Probe::new().read_from(&mut file).unwrap();

	assert_eq!(tagged_file.file_type(), &FileType::FLAC);
	assert!(tagged_file.first_tag().is_some());

	let mut tag = tagged_file.primary_tag_mut().unwrap();

	// We're replacing "Foo title"
	assert_eq!(
		tag.get_item_ref(&ItemKey::TrackTitle),
		Some(&TagItem::new(
			ItemKey::TrackTitle,
			ItemValue::Text(String::from("Foo title"))
		))
	);

	// Tag::insert_item returns a bool
	assert!(tag.insert_item(TagItem::new(
		ItemKey::TrackTitle,
		ItemValue::Text(String::from("Bar title"))
	)));

	assert!(tag.save_to(&mut file).is_ok());

	// Now reread the file
	let mut tagged_file = Probe::new().read_from(&mut file).unwrap();

	let mut tag = tagged_file.primary_tag_mut().unwrap();

	assert_eq!(
		tag.get_item_ref(&ItemKey::TrackTitle),
		Some(&TagItem::new(
			ItemKey::TrackTitle,
			ItemValue::Text(String::from("Bar title"))
		))
	);

	// Now set it back to "Foo title"
	assert!(tag.insert_item(TagItem::new(
		ItemKey::TrackTitle,
		ItemValue::Text(String::from("Foo title"))
	)));

	assert!(tag.save_to(&mut file).is_ok());
}
