#![cfg(feature = "default")]
use lofty::{MimeType, Picture, Tag};

macro_rules! full_test {
	($function:ident, $file:expr) => {
		#[test]
		fn $function() {
			add_tags!($file);
			remove_tags!($file);
		}
	};
}

macro_rules! add_tags {
	($file:expr) => {
		let mut tag = Tag::default().read_from_path($file).unwrap();

		tag.set_title("foo title");
		assert_eq!(tag.title(), Some("foo title"));

		tag.set_artist("foo artist");
		assert_eq!(tag.artist(), Some("foo artist"));

		tag.set_year(2020);
		assert_eq!(tag.year(), Some(2020));

		tag.set_album_title("foo album title");
		assert_eq!(tag.album_title(), Some("foo album title"));

		tag.set_album_artists("foo album artist".to_string());
		assert_eq!(tag.album_artists(), Some(vec!["foo album artist"]));

		// TODO
		// let cover = Picture {
		// 	mime_type: MimeType::Jpeg,
		// 	data: &vec![0u8; 10],
		// };
		//
		// tags.set_album_cover(cover.clone());
		// assert_eq!(tags.album_cover(), Some(cover));

		tag.write_to_path($file).unwrap();
	};
}

macro_rules! remove_tags {
	($file:expr) => {
		let mut tag = Tag::default().read_from_path($file).unwrap();
		assert_eq!(tag.title(), Some("foo title"));

		tag.remove_title();
		assert!(tag.title().is_none());
		tag.remove_title(); // should not panic

		tag.remove_artist();
		assert!(tag.artist().is_none());
		tag.remove_artist();

		tag.remove_year();
		assert!(tag.year().is_none());
		tag.remove_year();

		tag.remove_album_title();
		assert!(tag.album_title().is_none());
		tag.remove_album_title();

		tag.remove_album_artists();
		assert!(tag.album_artists().is_none());
		tag.remove_album_artists();

		// TODO
		// tags.remove_album_cover();
		// assert!(tags.album_cover().is_none());
		// tags.remove_album_cover();

		tag.write_to_path($file).unwrap();
	};
}

full_test!(test_ape, "tests/assets/a.ape");
full_test!(test_m4a, "tests/assets/a.m4a");
full_test!(test_mp3, "tests/assets/a.mp3");
full_test!(test_wav, "tests/assets/a.wav");

// Vorbis comments
full_test!(test_flac, "tests/assets/a.flac");
full_test!(test_ogg, "tests/assets/a.ogg");
full_test!(test_opus, "tests/assets/a.opus");
