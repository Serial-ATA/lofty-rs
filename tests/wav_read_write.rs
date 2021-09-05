use lofty::{FileType, ItemKey, ItemValue, Probe, TagItem, TagType};

#[test]
fn wav_read() {
	// Here we have a WAV file with both an ID3v2 chunk and a RIFF INFO chunk
	let file = Probe::new()
		.read_from_path("tests/assets/a_mixed.wav")
		.unwrap();

	assert_eq!(file.file_type(), &FileType::WAV);

	// Verify the ID3v2 tag first
	assert!(file.primary_tag().is_some());

	let tag = file.primary_tag().unwrap();

	// We have a title stored in here
	assert_eq!(tag.item_count(), 1);

	assert_eq!(
		tag.get_item_ref(&ItemKey::TrackTitle),
		Some(&TagItem::new(
			ItemKey::TrackTitle,
			ItemValue::Text(String::from("Foo title"))
		))
	);

	// Now verify the RIFF INFO chunk
	assert!(file.tag(&TagType::RiffInfo).is_some());

	let tag = file.tag(&TagType::RiffInfo).unwrap();

	// We also have a title stored in here
	assert_eq!(tag.item_count(), 1);

	assert_eq!(
		tag.get_item_ref(&ItemKey::TrackTitle),
		Some(&TagItem::new(
			ItemKey::TrackTitle,
			ItemValue::Text(String::from("Bar title"))
		))
	);
}

#[test]
fn wav_write() {
	let mut file = std::fs::OpenOptions::new()
		.read(true)
		.write(true)
		.open("tests/assets/a_mixed.wav")
		.unwrap();

	let mut tagged_file = Probe::new().read_from(&mut file).unwrap();

	assert_eq!(tagged_file.file_type(), &FileType::WAV);

	assert!(tagged_file.primary_tag().is_some());
	assert!(tagged_file.tag(&TagType::RiffInfo).is_some());

	// ID3v2
	let primary_tag = tagged_file.primary_tag_mut().unwrap();

	// We're replacing the title
	assert_eq!(
		primary_tag.get_item_ref(&ItemKey::TrackTitle),
		Some(&TagItem::new(
			ItemKey::TrackTitle,
			ItemValue::Text(String::from("Foo title"))
		))
	);

	// Tag::insert_item returns a bool
	assert!(primary_tag.insert_item(TagItem::new(
		ItemKey::TrackTitle,
		ItemValue::Text(String::from("Bar title"))
	)));

	// RIFF INFO
	let riff_info = tagged_file.tag_mut(&TagType::RiffInfo).unwrap();

	assert_eq!(
		riff_info.get_item_ref(&ItemKey::TrackTitle),
		Some(&TagItem::new(
			ItemKey::TrackTitle,
			ItemValue::Text(String::from("Bar title"))
		))
	);

	assert!(riff_info.insert_item(TagItem::new(
		ItemKey::TrackTitle,
		ItemValue::Text(String::from("Baz title"))
	)));

	// TODO
	assert!(riff_info.save_to(&mut file).is_ok());

	// Now reread the file
	let mut tagged_file = Probe::new().read_from(&mut file).unwrap();

	let primary_tag = tagged_file.primary_tag_mut().unwrap();

	// TODO
	// assert_eq!(
	// 	primary_tag.get_item_ref(&ItemKey::TrackTitle),
	// 	Some(&TagItem::new(
	// 		ItemKey::TrackTitle,
	// 		ItemValue::Text(String::from("Bar title"))
	// 	))
	// );

	// Now set them back
	assert!(primary_tag.insert_item(TagItem::new(
		ItemKey::TrackTitle,
		ItemValue::Text(String::from("Foo title"))
	)));

	let riff_info = tagged_file.tag_mut(&TagType::RiffInfo).unwrap();

	assert_eq!(
		riff_info.get_item_ref(&ItemKey::TrackTitle),
		Some(&TagItem::new(
			ItemKey::TrackTitle,
			ItemValue::Text(String::from("Baz title"))
		))
	);

	assert!(riff_info.insert_item(TagItem::new(
		ItemKey::TrackTitle,
		ItemValue::Text(String::from("Bar title"))
	)));

	// TODO
	assert!(riff_info.save_to(&mut file).is_ok());
}
