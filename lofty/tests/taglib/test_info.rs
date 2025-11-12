use lofty::iff::wav::RiffInfoList;
use lofty::tag::Accessor;
use lofty::tag::items::Timestamp;

#[test_log::test]
fn test_title() {
	let mut tag = RiffInfoList::default();

	assert!(tag.title().is_none());
	tag.set_title(String::from("Test title 1"));
	tag.insert(String::from("TEST"), String::from("Dummy Text"));

	assert_eq!(tag.title().as_deref(), Some("Test title 1"));
	assert_eq!(tag.get("INAM"), Some("Test title 1"));
	assert_eq!(tag.get("TEST"), Some("Dummy Text"));
}

#[test_log::test]
fn test_numeric_fields() {
	let mut tag = RiffInfoList::default();

	assert!(tag.track().is_none());
	tag.set_track(1234);
	assert_eq!(tag.track(), Some(1234));
	assert_eq!(tag.get("IPRT"), Some("1234"));

	assert!(tag.date().is_none());
	tag.set_date(Timestamp {
		year: 1234,
		..Timestamp::default()
	});
	assert_eq!(tag.date().map(|date| date.year), Some(1234));
	assert_eq!(tag.get("ICRD"), Some("1234"));
}
