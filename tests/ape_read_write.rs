use lofty::id3::Id3v2Version;
use lofty::{FileType, ItemKey, ItemValue, Probe, TagItem, TagType};

#[test]
fn ape_test() {
	println!("APE: Reading");
	ape_read();
	println!("APE: Writing");
	ape_write();
}

fn ape_read() {
	// Here we have an APE file with an ID3v2, ID3v1, and an APEv2 tag
	let file = Probe::new().read_from_path("tests/assets/a.ape").unwrap();

	assert_eq!(file.file_type(), &FileType::APE);

	// Verify the APEv2 tag first
	assert!(file.primary_tag().is_some());

	let tag = file.primary_tag().unwrap();

	// We have an artist stored in here
	assert_eq!(tag.item_count(), 1);

	assert_eq!(
		tag.get_item_ref(&ItemKey::TrackArtist),
		Some(&TagItem::new(
			ItemKey::TrackArtist,
			ItemValue::Text(String::from("Foo artist"))
		))
	);

	// Now verify ID3v1
	assert!(file.tag(&TagType::Id3v1).is_some());

	let tag = file.tag(&TagType::Id3v1).unwrap();

	// We also have an artist stored in here
	assert_eq!(tag.item_count(), 1);

	assert_eq!(
		tag.get_item_ref(&ItemKey::TrackArtist),
		Some(&TagItem::new(
			ItemKey::TrackArtist,
			ItemValue::Text(String::from("Bar artist"))
		))
	);

	// Finally, verify ID3v2
	assert!(file.tag(&TagType::Id3v2(Id3v2Version::V4)).is_some());

	let tag = file.tag(&TagType::Id3v2(Id3v2Version::V4)).unwrap();

	// We also have an artist stored in here
	assert_eq!(tag.item_count(), 1);

	assert_eq!(
		tag.get_item_ref(&ItemKey::TrackArtist),
		Some(&TagItem::new(
			ItemKey::TrackArtist,
			ItemValue::Text(String::from("Baz artist"))
		))
	);
}

fn ape_write() {
	let mut file = std::fs::OpenOptions::new()
		.read(true)
		.write(true)
		.open("tests/assets/a.ape")
		.unwrap();

	let mut tagged_file = Probe::new().read_from(&mut file).unwrap();

	assert_eq!(tagged_file.file_type(), &FileType::APE);

	assert!(tagged_file.primary_tag().is_some());
	assert!(tagged_file.tag(&TagType::Id3v1).is_some());
	assert!(tagged_file.tag(&TagType::Id3v2(Id3v2Version::V4)).is_some());

	// APEv2
	let primary_tag = tagged_file.primary_tag_mut().unwrap();

	// We're replacing the artists
	assert_eq!(
		primary_tag.get_item_ref(&ItemKey::TrackArtist),
		Some(&TagItem::new(
			ItemKey::TrackArtist,
			ItemValue::Text(String::from("Foo artist"))
		))
	);

	// Tag::insert_item returns a bool
	assert!(primary_tag.insert_item(TagItem::new(
		ItemKey::TrackArtist,
		ItemValue::Text(String::from("Bar artist"))
	)));

	assert!(primary_tag.save_to(&mut file).is_ok());

	// ID3v1
	let id3v1 = tagged_file.tag_mut(&TagType::Id3v1).unwrap();

	assert_eq!(
		id3v1.get_item_ref(&ItemKey::TrackArtist),
		Some(&TagItem::new(
			ItemKey::TrackArtist,
			ItemValue::Text(String::from("Bar artist"))
		))
	);

	id3v1.insert_item_unchecked(TagItem::new(
		ItemKey::TrackArtist,
		ItemValue::Text(String::from("Baz artist")),
	));

	// ID3v2
	// let id3v2 = tagged_file.tag_mut(&TagType::Id3v1).unwrap();
	//
	// assert_eq!(
	//     id3v2.get_item_ref(&ItemKey::TrackArtist),
	//     Some(&TagItem::new(
	//         ItemKey::TrackArtist,
	//         ItemValue::Text(String::from("Baz artist"))
	//     ))
	// );
	//
	// assert!(id3v2.insert_item(TagItem::new(
	//     ItemKey::TrackArtist,
	//     ItemValue::Text(String::from("Qux artist"))
	// )));

	// TODO
	assert!(id3v1.save_to(&mut file).is_ok());

	// Now reread the file
	let mut tagged_file = Probe::new().read_from(&mut file).unwrap();

	let primary_tag = tagged_file.primary_tag_mut().unwrap();

	assert_eq!(
		primary_tag.get_item_ref(&ItemKey::TrackArtist),
		Some(&TagItem::new(
			ItemKey::TrackArtist,
			ItemValue::Text(String::from("Bar artist"))
		))
	);

	// Now set them back
	assert!(primary_tag.insert_item(TagItem::new(
		ItemKey::TrackArtist,
		ItemValue::Text(String::from("Foo artist"))
	)));

	assert!(primary_tag.save_to(&mut file).is_ok());

	let id3v1 = tagged_file.tag_mut(&TagType::Id3v1).unwrap();

	assert_eq!(
		id3v1.get_item_ref(&ItemKey::TrackArtist),
		Some(&TagItem::new(
			ItemKey::TrackArtist,
			ItemValue::Text(String::from("Baz artist"))
		))
	);

	id3v1.insert_item_unchecked(TagItem::new(
		ItemKey::TrackArtist,
		ItemValue::Text(String::from("Bar artist")),
	));

	// let id3v2 = tagged_file.tag_mut(&TagType::Id3v2(Id3v2Version::V4)).unwrap();
	//
	// assert_eq!(
	//     id3v2.get_item_ref(&ItemKey::TrackArtist),
	//     Some(&TagItem::new(
	//         ItemKey::TrackArtist,
	//         ItemValue::Text(String::from("Qux artist"))
	//     ))
	// );
	//
	// assert!(id3v2.insert_item(TagItem::new(
	//     ItemKey::TrackArtist,
	//     ItemValue::Text(String::from("Baz artist"))
	// )));

	// TODO
	assert!(id3v1.save_to(&mut file).is_ok());
}
