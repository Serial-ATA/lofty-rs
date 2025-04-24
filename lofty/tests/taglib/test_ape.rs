use crate::temp_file;
use crate::util::get_file;

use std::io::Seek;
use std::time::Duration;

use lofty::ape::{ApeFile, ApeItem, ApeTag};
use lofty::config::{ParseOptions, WriteOptions};
use lofty::file::{AudioFile, FileType};
use lofty::id3::v1::Id3v1Tag;
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemValue, TagExt};

fn test_399(path: &str) {
	let f = get_file::<ApeFile>(path);
	let properties = f.properties();

	assert_eq!(properties.duration(), Duration::from_millis(3550));
	assert_eq!(properties.bitrate(), 192);
	assert_eq!(properties.channels(), 2);
	assert_eq!(properties.sample_rate(), 44100);
	assert_eq!(properties.bit_depth(), 16);
	// TODO
	// assert_eq!(properties.sample_frames(), 156556);
	assert_eq!(properties.version(), 3990)
}

#[test_log::test]
fn test_properties_399() {
	test_399("tests/taglib/data/mac-399.ape")
}

#[test_log::test]
fn test_properties_399_tagged() {
	test_399("tests/taglib/data/mac-399-tagged.ape")
}

#[test_log::test]
fn test_properties_399_id3v2() {
	test_399("tests/taglib/data/mac-399-id3v2.ape")
}

#[test_log::test]
fn test_properties_396() {
	let f = get_file::<ApeFile>("tests/taglib/data/mac-396.ape");
	let properties = f.properties();

	assert_eq!(properties.duration(), Duration::from_millis(3685));
	assert_eq!(properties.bitrate(), 0);
	assert_eq!(properties.channels(), 2);
	assert_eq!(properties.sample_rate(), 44100);
	assert_eq!(properties.bit_depth(), 16);
	// TODO
	// assert_eq!(properties.sample_frames(), 162496);
	assert_eq!(properties.version(), 3960)
}

#[test_log::test]
fn test_properties_390() {
	let f = get_file::<ApeFile>("tests/taglib/data/mac-390-hdr.ape");
	let properties = f.properties();

	assert_eq!(properties.duration(), Duration::from_millis(15630));
	assert_eq!(properties.bitrate(), 0);
	assert_eq!(properties.channels(), 2);
	assert_eq!(properties.sample_rate(), 44100);
	assert_eq!(properties.bit_depth(), 16);
	// TODO
	// assert_eq!(properties.sample_frames(), 689262);
	assert_eq!(properties.version(), 3900)
}

#[test_log::test]
fn test_fuzzed_file_1() {
	assert_eq!(
		Probe::open("tests/taglib/data/longloop.ape")
			.unwrap()
			.guess_file_type()
			.unwrap()
			.file_type(),
		Some(FileType::Ape)
	);
}

#[test_log::test]
fn test_fuzzed_file_2() {
	assert_eq!(
		Probe::open("tests/taglib/data/zerodiv.ape")
			.unwrap()
			.guess_file_type()
			.unwrap()
			.file_type(),
		Some(FileType::Ape)
	);
}

#[test_log::test]
fn test_strip_and_properties() {
	let mut file = temp_file!("tests/taglib/data/mac-399.ape");
	{
		let mut ape_file = ApeFile::read_from(&mut file, ParseOptions::new()).unwrap();

		let mut ape_tag = ApeTag::default();
		ape_tag.set_title(String::from("APE"));
		ape_file.set_ape(ape_tag);

		let mut id3v1_tag = Id3v1Tag::default();
		id3v1_tag.set_title(String::from("ID3v1"));
		ape_file.set_id3v1(id3v1_tag);

		file.rewind().unwrap();
		ape_file
			.save_to(&mut file, WriteOptions::default())
			.unwrap();
	}
	{
		file.rewind().unwrap();
		let mut ape_file = ApeFile::read_from(&mut file, ParseOptions::new()).unwrap();

		assert_eq!(ape_file.ape().unwrap().title().as_deref(), Some("APE"));
		ape_file.remove_ape();

		assert_eq!(ape_file.id3v1().unwrap().title().as_deref(), Some("ID3v1"));
		ape_file.remove_id3v1();

		assert!(!ape_file.contains_tag());
	}
}

#[test_log::test]
fn test_properties() {
	let mut tag = ApeTag::default();
	tag.insert(
		ApeItem::new(
			String::from("ALBUM"),
			ItemValue::Text(String::from("Album")),
		)
		.unwrap(),
	);
	tag.insert(
		ApeItem::new(
			String::from("ALBUMARTIST"),
			ItemValue::Text(String::from("Album Artist")),
		)
		.unwrap(),
	);
	tag.insert(
		ApeItem::new(
			String::from("ALBUMARTISTSORT"),
			ItemValue::Text(String::from("Album Artist Sort")),
		)
		.unwrap(),
	);
	tag.insert(
		ApeItem::new(
			String::from("ALBUMSORT"),
			ItemValue::Text(String::from("Album Sort")),
		)
		.unwrap(),
	);
	tag.insert(
		ApeItem::new(
			String::from("ARTIST"),
			ItemValue::Text(String::from("Artist")),
		)
		.unwrap(),
	);
	tag.insert(
		ApeItem::new(
			String::from("ARTISTS"),
			ItemValue::Text(String::from("Artists")),
		)
		.unwrap(),
	);
	tag.insert(
		ApeItem::new(
			String::from("ARTISTSORT"),
			ItemValue::Text(String::from("Artist Sort")),
		)
		.unwrap(),
	);
	tag.insert(ApeItem::new(String::from("ASIN"), ItemValue::Text(String::from("ASIN"))).unwrap());
	tag.insert(
		ApeItem::new(
			String::from("BARCODE"),
			ItemValue::Text(String::from("Barcode")),
		)
		.unwrap(),
	);
	tag.insert(
		ApeItem::new(
			String::from("CATALOGNUMBER"),
			ItemValue::Text(String::from("Catalog Number 1")),
		)
		.unwrap(),
	);
	tag.insert(
		ApeItem::new(
			String::from("COMMENT"),
			ItemValue::Text(String::from("Comment")),
		)
		.unwrap(),
	);
	tag.insert(
		ApeItem::new(
			String::from("DATE"),
			ItemValue::Text(String::from("2021-01-10")),
		)
		.unwrap(),
	);
	tag.insert(
		ApeItem::new(
			String::from("DISCNUMBER"),
			ItemValue::Text(String::from("3/5")),
		)
		.unwrap(),
	);
	tag.insert(
		ApeItem::new(
			String::from("GENRE"),
			ItemValue::Text(String::from("Genre")),
		)
		.unwrap(),
	);
	tag.insert(
		ApeItem::new(
			String::from("ISRC"),
			ItemValue::Text(String::from("UKAAA0500001")),
		)
		.unwrap(),
	);
	tag.insert(
		ApeItem::new(
			String::from("LABEL"),
			ItemValue::Text(String::from("Label 1")),
		)
		.unwrap(),
	);
	tag.insert(
		ApeItem::new(
			String::from("MEDIA"),
			ItemValue::Text(String::from("Media")),
		)
		.unwrap(),
	);
	tag.insert(
		ApeItem::new(
			String::from("MUSICBRAINZ_ALBUMARTISTID"),
			ItemValue::Text(String::from("MusicBrainz_AlbumartistID")),
		)
		.unwrap(),
	);
	tag.insert(
		ApeItem::new(
			String::from("MUSICBRAINZ_ALBUMID"),
			ItemValue::Text(String::from("MusicBrainz_AlbumID")),
		)
		.unwrap(),
	);
	tag.insert(
		ApeItem::new(
			String::from("MUSICBRAINZ_ARTISTID"),
			ItemValue::Text(String::from("MusicBrainz_ArtistID")),
		)
		.unwrap(),
	);
	tag.insert(
		ApeItem::new(
			String::from("MUSICBRAINZ_RELEASEGROUPID"),
			ItemValue::Text(String::from("MusicBrainz_ReleasegroupID")),
		)
		.unwrap(),
	);
	tag.insert(
		ApeItem::new(
			String::from("MUSICBRAINZ_RELEASETRACKID"),
			ItemValue::Text(String::from("MusicBrainz_ReleasetrackID")),
		)
		.unwrap(),
	);
	tag.insert(
		ApeItem::new(
			String::from("MUSICBRAINZ_TRACKID"),
			ItemValue::Text(String::from("MusicBrainz_TrackID")),
		)
		.unwrap(),
	);
	tag.insert(
		ApeItem::new(
			String::from("ORIGINALDATE"),
			ItemValue::Text(String::from("2021-01-09")),
		)
		.unwrap(),
	);
	tag.insert(
		ApeItem::new(
			String::from("RELEASECOUNTRY"),
			ItemValue::Text(String::from("Release Country")),
		)
		.unwrap(),
	);
	tag.insert(
		ApeItem::new(
			String::from("RELEASESTATUS"),
			ItemValue::Text(String::from("Release Status")),
		)
		.unwrap(),
	);
	tag.insert(
		ApeItem::new(
			String::from("RELEASETYPE"),
			ItemValue::Text(String::from("Release Type")),
		)
		.unwrap(),
	);
	tag.insert(
		ApeItem::new(
			String::from("SCRIPT"),
			ItemValue::Text(String::from("Script")),
		)
		.unwrap(),
	);
	tag.insert(
		ApeItem::new(
			String::from("TITLE"),
			ItemValue::Text(String::from("Title")),
		)
		.unwrap(),
	);
	tag.insert(
		ApeItem::new(
			String::from("TRACKNUMBER"),
			ItemValue::Text(String::from("2/3")),
		)
		.unwrap(),
	);

	let mut file = temp_file!("tests/taglib/data/mac-399.ape");
	{
		let mut ape_file = ApeFile::read_from(&mut file, ParseOptions::new()).unwrap();

		ape_file.set_ape(tag.clone());

		file.rewind().unwrap();
		ape_file
			.ape()
			.unwrap()
			.save_to(&mut file, WriteOptions::default())
			.unwrap();
	}
	{
		file.rewind().unwrap();
		let ape_file = ApeFile::read_from(&mut file, ParseOptions::new()).unwrap();

		assert_eq!(ape_file.ape(), Some(&tag));
	}
}

#[test_log::test]
fn test_repeated_save() {
	let mut file = temp_file!("tests/taglib/data/mac-399.ape");
	{
		let mut ape_file = ApeFile::read_from(&mut file, ParseOptions::new()).unwrap();
		assert!(ape_file.ape().is_none());
		assert!(ape_file.id3v1().is_none());

		let mut ape_tag = ApeTag::default();
		ape_tag.set_title(String::from("01234 56789 ABCDE FGHIJ"));
		ape_file.set_ape(ape_tag);
		ape_file
			.save_to(&mut file, WriteOptions::default())
			.unwrap();
		file.rewind().unwrap();

		ape_file.ape_mut().unwrap().set_title(String::from("0"));
		ape_file
			.save_to(&mut file, WriteOptions::default())
			.unwrap();
		file.rewind().unwrap();

		let mut id3v1_tag = Id3v1Tag::default();
		id3v1_tag.set_title(String::from("01234 56789 ABCDE FGHIJ"));
		ape_file.set_id3v1(id3v1_tag);
		ape_file.ape_mut().unwrap().set_title(String::from(
			"01234 56789 ABCDE FGHIJ 01234 56789 ABCDE FGHIJ 01234 56789",
		));
		ape_file
			.save_to(&mut file, WriteOptions::default())
			.unwrap();
	}
	{
		file.rewind().unwrap();
		let ape_file = ApeFile::read_from(&mut file, ParseOptions::new()).unwrap();

		assert!(ape_file.ape().is_some());
		assert!(ape_file.id3v1().is_some());
	}
}
