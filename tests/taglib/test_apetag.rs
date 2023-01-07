use crate::temp_file;

use std::io::Seek;

use lofty::ape::{ApeFile, ApeItem, ApeTag};
use lofty::{Accessor, AudioFile, ItemValue, ParseOptions, TagExt};

#[test]
fn test_is_empty() {
	let mut tag = ApeTag::default();
	assert!(tag.is_empty());
	tag.insert(
		ApeItem::new(
			String::from("COMPOSER"),
			ItemValue::Text(String::from("Mike Oldfield")),
		)
		.unwrap(),
	);
	assert!(!tag.is_empty());
}

#[test]
fn test_is_empty_2() {
	let mut tag = ApeTag::default();
	assert!(tag.is_empty());
	tag.set_artist(String::from("Mike Oldfield"));
	assert!(!tag.is_empty());
}

#[test]
#[ignore]
fn test_property_interface_1() {
	// Marker test, Lofty does not replicate the TagLib property API
}

#[test]
#[ignore]
fn test_property_interface_2() {
	// Marker test, Lofty does not replicate the TagLib property API
}

#[test]
fn test_invalid_keys() {
	static INVALID_KEY_ONE_CHARACTER: &str = "A";
	static INVALID_KEY_FORBIDDEN_STRING: &str = "MP+";
	static INVALID_KEY_UNICODE: &str = "\x1234\x3456";
	static VALID_KEY_SPACE_AND_TILDE: &str = "A B~C";
	static VALID_KEY_NORMAL_ONE: &str = "ARTIST";

	assert!(ApeItem::new(
		String::from(INVALID_KEY_ONE_CHARACTER),
		ItemValue::Text(String::from("invalid key: one character"))
	)
	.is_err());
	assert!(ApeItem::new(
		String::from(INVALID_KEY_FORBIDDEN_STRING),
		ItemValue::Text(String::from("invalid key: forbidden string"))
	)
	.is_err());
	assert!(ApeItem::new(
		String::from(INVALID_KEY_UNICODE),
		ItemValue::Text(String::from("invalid key: Unicode"))
	)
	.is_err());

	let valid_space_and_tilde = ApeItem::new(
		String::from(VALID_KEY_SPACE_AND_TILDE),
		ItemValue::Text(String::from("valid key: space and tilde")),
	);
	assert!(valid_space_and_tilde.is_ok());

	let valid_normal_one = ApeItem::new(
		String::from(VALID_KEY_NORMAL_ONE),
		ItemValue::Text(String::from("valid key: normal one")),
	);
	assert!(valid_normal_one.is_ok());

	let mut tag = ApeTag::default();
	tag.insert(valid_space_and_tilde.unwrap());
	tag.insert(valid_normal_one.unwrap());
	assert_eq!(tag.items().len(), 2);
}

#[test]
#[ignore]
fn test_text_binary() {
	// Marker test, this is useless as Lofty does not have a similar API that the test is based upon:
	// https://github.com/taglib/taglib/blob/a31356e330674640a07bef7d71d08242cae8e9bf/tests/test_apetag.cpp#L153
}

// TODO: Does not work! We fall for this collision.
#[test]
#[ignore]
fn test_id3v1_collision() {
	// TODO: This uses a musepack file in the TagLib test suite. It makes no difference for this test, but should
	//       be changed once Musepack is supported in Lofty.
	let mut file = temp_file!("tests/taglib/data/no-tags.ape");
	{
		let mut ape_file =
			ApeFile::read_from(&mut file, ParseOptions::new().read_properties(false)).unwrap();
		assert!(ape_file.ape().is_none());
		assert!(ape_file.id3v1().is_none());

		let mut ape_tag = ApeTag::default();
		ape_tag.set_artist(String::from("Filltointersect    "));
		ape_tag.set_title(String::from("Filltointersect    "));
		ape_file.set_ape(ape_tag);
		ape_file.save_to(&mut file).unwrap();
	}
	{
		file.rewind().unwrap();
		let ape_file =
			ApeFile::read_from(&mut file, ParseOptions::new().read_properties(false)).unwrap();
		assert!(ape_file.id3v1().is_none());
	}
}
