use lofty::{FileType, ItemKey, ItemValue, Probe, TagItem, TagType};

#[test]
fn aiff_read() {
	// Here we have an AIFF file with both an ID3v2 chunk and text chunks
	let file = Probe::new()
		.read_from_path("tests/assets/a_mixed.aiff")
		.unwrap();

	assert_eq!(file.file_type(), &FileType::AIFF);

	// Verify the ID3v2 tag first
	assert!(file.primary_tag().is_some());

	let tag = file.primary_tag().unwrap();

	// We have an artist stored in here
	assert_eq!(tag.item_count(), 1);

	assert_eq!(
		tag.get_item_ref(&ItemKey::TrackArtist),
		Some(&TagItem::new(
			ItemKey::TrackArtist,
			ItemValue::Text(String::from("Bar artist"))
		))
	);

	// Now verify the text chunks
	assert!(file.tag(&TagType::AiffText).is_some());

	let tag = file.tag(&TagType::AiffText).unwrap();

	// We also have an artist stored in here
	assert_eq!(tag.item_count(), 1);

	assert_eq!(
		tag.get_item_ref(&ItemKey::TrackArtist),
		Some(&TagItem::new(
			ItemKey::TrackArtist,
			ItemValue::Text(String::from("Foo artist"))
		))
	);
}

#[test]
fn aiff_write() {
	let mut file = std::fs::OpenOptions::new()
		.read(true)
		.write(true)
		.open("tests/assets/a_mixed.aiff")
		.unwrap();

	let mut tagged_file = Probe::new().read_from(&mut file).unwrap();

	assert_eq!(tagged_file.file_type(), &FileType::AIFF);

	assert!(tagged_file.primary_tag().is_some());
	assert!(tagged_file.tag(&TagType::AiffText).is_some());

	// ID3v2
	let primary_tag = tagged_file.primary_tag_mut().unwrap();

	// We're replacing the artists
	assert_eq!(
		primary_tag.get_item_ref(&ItemKey::TrackArtist),
		Some(&TagItem::new(
			ItemKey::TrackArtist,
			ItemValue::Text(String::from("Bar artist"))
		))
	);

	// Tag::insert_item returns a bool
	assert!(primary_tag.insert_item(TagItem::new(
		ItemKey::TrackArtist,
		ItemValue::Text(String::from("Baz artist"))
	)));

	// Text chunks
	let text_chunks = tagged_file.tag_mut(&TagType::AiffText).unwrap();

	assert_eq!(
		text_chunks.get_item_ref(&ItemKey::TrackArtist),
		Some(&TagItem::new(
			ItemKey::TrackArtist,
			ItemValue::Text(String::from("Foo artist"))
		))
	);

	assert!(text_chunks.insert_item(TagItem::new(
		ItemKey::TrackArtist,
		ItemValue::Text(String::from("Bar artist"))
	)));

	// TODO
	assert!(text_chunks.save_to(&mut file).is_ok());

	// Now reread the file
	let mut tagged_file = Probe::new().read_from(&mut file).unwrap();

	let primary_tag = tagged_file.primary_tag_mut().unwrap();

	// TODO
	// assert_eq!(
	// 	primary_tag.get_item_ref(&ItemKey::TrackArtist),
	// 	Some(&TagItem::new(
	// 		ItemKey::TrackArtist,
	// 		ItemValue::Text(String::from("Baz artist"))
	// 	))
	// );

	// Now set them back
	assert!(primary_tag.insert_item(TagItem::new(
		ItemKey::TrackArtist,
		ItemValue::Text(String::from("Bar artist"))
	)));

	let text_chunks = tagged_file.tag_mut(&TagType::AiffText).unwrap();

	assert_eq!(
		text_chunks.get_item_ref(&ItemKey::TrackArtist),
		Some(&TagItem::new(
			ItemKey::TrackArtist,
			ItemValue::Text(String::from("Bar artist"))
		))
	);

	assert!(text_chunks.insert_item(TagItem::new(
		ItemKey::TrackArtist,
		ItemValue::Text(String::from("Foo artist"))
	)));

	// TODO
	assert!(text_chunks.save_to(&mut file).is_ok());
}
