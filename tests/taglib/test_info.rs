use lofty::iff::wav::RIFFInfoList;
use lofty::Accessor;

#[test]
fn test_title() {
	let mut tag = RIFFInfoList::default();

	assert!(tag.title().is_none());
	tag.set_title(String::from("Test title 1"));
	tag.insert(String::from("TEST"), String::from("Dummy Text"));

	assert_eq!(tag.title().as_deref(), Some("Test title 1"));
	assert_eq!(tag.get("INAM"), Some("Test title 1"));
	assert_eq!(tag.get("TEST"), Some("Dummy Text"));
}

#[test]
fn test_numeric_fields() {
	let mut tag = RIFFInfoList::default();

	assert!(tag.track().is_none());
	tag.set_track(1234);
	assert_eq!(tag.track(), Some(1234));
	assert_eq!(tag.get("IPRT"), Some("1234"));

	assert!(tag.year().is_none());
	tag.set_year(1234);
	assert_eq!(tag.year(), Some(1234));
	assert_eq!(tag.get("ICRD"), Some("1234"));
}
