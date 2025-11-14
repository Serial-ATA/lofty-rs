use crate::temp_file;

use std::io::Seek;

use lofty::ape::{ApeItem, ApeTag};
use lofty::config::{ParseOptions, WriteOptions};
use lofty::file::AudioFile;
use lofty::musepack::MpcFile;
use lofty::tag::{Accessor, ItemValue, TagExt};

#[test_log::test]
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

#[test_log::test]
fn test_is_empty_2() {
	let mut tag = ApeTag::default();
	assert!(tag.is_empty());
	tag.set_artist(String::from("Mike Oldfield"));
	assert!(!tag.is_empty());
}

#[test_log::test]
#[ignore = "Marker test, Lofty does not replicate the TagLib property API"]
fn test_property_interface_1() {}

#[test_log::test]
#[ignore = "Marker test, Lofty does not replicate the TagLib property API"]
fn test_property_interface_2() {}

#[test_log::test]
fn test_invalid_keys() {
	static INVALID_KEY_ONE_CHARACTER: &str = "A";
	static INVALID_KEY_FORBIDDEN_STRING: &str = "MP+";
	static INVALID_KEY_UNICODE: &str = "\x1234\x3456";
	static VALID_KEY_SPACE_AND_TILDE: &str = "A B~C";
	static VALID_KEY_NORMAL_ONE: &str = "ARTIST";

	assert!(
		ApeItem::new(
			String::from(INVALID_KEY_ONE_CHARACTER),
			ItemValue::Text(String::from("invalid key: one character"))
		)
		.is_err()
	);
	assert!(
		ApeItem::new(
			String::from(INVALID_KEY_FORBIDDEN_STRING),
			ItemValue::Text(String::from("invalid key: forbidden string"))
		)
		.is_err()
	);
	assert!(
		ApeItem::new(
			String::from(INVALID_KEY_UNICODE),
			ItemValue::Text(String::from("invalid key: Unicode"))
		)
		.is_err()
	);

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
	assert_eq!(tag.len(), 2);
}

#[test_log::test]
#[ignore = "Marker test, Lofty doesn't replicate this API"]
fn test_text_binary() {
	// https://github.com/taglib/taglib/blob/a31356e330674640a07bef7d71d08242cae8e9bf/tests/test_apetag.cpp#L153
}

// TODO: Does not work! We fall for this collision.
#[test_log::test]
#[ignore = "We currently fall for this collision"]
fn test_id3v1_collision() {
	let mut file = temp_file!("tests/taglib/data/no-tags.mpc");
	{
		let mut mpc_file =
			MpcFile::read_from(&mut file, ParseOptions::new().read_properties(false)).unwrap();
		assert!(mpc_file.ape().is_none());
		assert!(mpc_file.id3v1().is_none());

		let mut ape_tag = ApeTag::default();
		ape_tag.set_artist(String::from("Filltointersect    "));
		ape_tag.set_title(String::from("Filltointersect    "));
		mpc_file.set_ape(ape_tag);
		mpc_file
			.save_to(&mut file, WriteOptions::default())
			.unwrap();
	}
	{
		file.rewind().unwrap();
		let mpc_file =
			MpcFile::read_from(&mut file, ParseOptions::new().read_properties(false)).unwrap();
		assert!(mpc_file.id3v1().is_none());
	}
}
