use lofty::{FileType, ItemKey, ItemValue, Probe, Tag, TagItem};

// The tests for OGG Opus/Vorbis are nearly identical

#[test]
fn ogg_opus_read() {
	let file = Probe::new().read_from_path("tests/assets/a.opus").unwrap();

	assert_eq!(file.file_type(), &FileType::Opus);

	// In the case of Opus, a metadata packet is mandatory anyway
	assert!(file.first_tag().is_some());

	// primary and first tag are identical for Opus
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
fn ogg_opus_write() {
	let mut file = std::fs::OpenOptions::new()
		.read(true)
		.write(true)
		.open("tests/assets/a.opus")
		.unwrap();

	let mut tagged_file = Probe::new().read_from(&mut file).unwrap();

	assert_eq!(tagged_file.file_type(), &FileType::Opus);
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

#[test]
fn ogg_vorbis_read() {
	let file = Probe::new().read_from_path("tests/assets/a.ogg").unwrap();

	assert_eq!(file.file_type(), &FileType::Vorbis);

	// In the case of OGG Vorbis, a metadata packet is mandatory anyway
	assert!(file.first_tag().is_some());

	// primary and first tag are identical for OGG Vorbis
	let tag = file.primary_tag().unwrap();

	// We have the vendor string and a title stored in the tag
	assert_eq!(tag.item_count(), 2);

	assert_eq!(
		tag.get_item_ref(&ItemKey::TrackTitle),
		Some(&TagItem::new(
			ItemKey::TrackTitle,
			ItemValue::Text(String::from("Bar title"))
		))
	);
}

#[test]
fn ogg_vorbis_write() {
	let mut file = std::fs::OpenOptions::new()
		.read(true)
		.write(true)
		.open("tests/assets/a.ogg")
		.unwrap();

	let mut tagged_file = Probe::new().read_from(&mut file).unwrap();

	assert_eq!(tagged_file.file_type(), &FileType::Vorbis);
	assert!(tagged_file.first_tag().is_some());

	let mut tag = tagged_file.primary_tag_mut().unwrap();

	// We're replacing "Bar title"
	assert_eq!(
		tag.get_item_ref(&ItemKey::TrackTitle),
		Some(&TagItem::new(
			ItemKey::TrackTitle,
			ItemValue::Text(String::from("Bar title"))
		))
	);

	// Tag::insert_item returns a bool
	assert!(tag.insert_item(TagItem::new(
		ItemKey::TrackTitle,
		ItemValue::Text(String::from("Baz title"))
	)));

	assert!(tag.save_to(&mut file).is_ok());

	// Now reread the file
	let mut tagged_file = Probe::new().read_from(&mut file).unwrap();

	let mut tag = tagged_file.primary_tag_mut().unwrap();

	assert_eq!(
		tag.get_item_ref(&ItemKey::TrackTitle),
		Some(&TagItem::new(
			ItemKey::TrackTitle,
			ItemValue::Text(String::from("Baz title"))
		))
	);

	// Now set it back to "Foo title"
	assert!(tag.insert_item(TagItem::new(
		ItemKey::TrackTitle,
		ItemValue::Text(String::from("Bar title"))
	)));

	assert!(tag.save_to(&mut file).is_ok());
}
