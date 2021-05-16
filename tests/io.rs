#![cfg(feature = "default")]
use lofty::{MimeType, Picture, PictureType, Tag};

macro_rules! full_test {
	($function:ident, $file:expr) => {
		#[test]
		fn $function() {
			println!("-- Adding tags --");
			add_tags!($file);
			println!("-- Verifying tags --");
			verify_write!($file);
			println!("-- Removing tags --");
			remove_tags!($file);
		}
	};
}

macro_rules! add_tags {
	($file:expr) => {
		println!("Reading file");
		let mut tag = Tag::default().read_from_path_signature($file).unwrap();

		println!("Setting title");
		tag.set_title("foo title");

		println!("Setting artist");
		tag.set_artist("foo artist");

		println!("Setting year");
		tag.set_year(2020);

		println!("Setting album title");
		tag.set_album_title("foo album title");

		println!("Setting album artists");
		tag.set_album_artist("foo album artist");

		let covers = (
			Picture {
				pic_type: PictureType::CoverFront,
				mime_type: MimeType::Jpeg,
				data: vec![0; 10],
			},
			Picture {
				pic_type: PictureType::CoverBack,
				mime_type: MimeType::Jpeg,
				data: vec![0; 11],
			},
		);

		let file_name = stringify!($file);

		if file_name != stringify!("tests/assets/a.wav") {
			println!("Setting front cover");
			tag.set_front_cover(covers.0.clone());
			assert_eq!(tag.front_cover(), Some(covers.0));

			println!("Setting back cover");
			tag.set_back_cover(covers.1.clone());
			assert_eq!(tag.back_cover(), Some(covers.1));
		}

		println!("Writing");
		tag.write_to_path($file).unwrap();
	};
}

macro_rules! verify_write {
	($file:expr) => {
		println!("Reading file");
		let tag = Tag::default().read_from_path_signature($file).unwrap();

		let file_name = stringify!($file);

		println!("Verifying title");
		assert_eq!(tag.title(), Some("foo title"));

		println!("Verifying artist");
		assert_eq!(tag.artist_str(), Some("foo artist"));

		// Skip this since RIFF INFO doesn't support year
		if file_name != stringify!("tests/assets/a.wav") {
			println!("Verifying year");
			assert_eq!(tag.year(), Some(2020));
		}

		println!("Verifying album title");
		assert_eq!(tag.album_title(), Some("foo album title"));

		// Skip this since RIFF INFO doesn't guarantee album artist
		if file_name != stringify!("tests/assets/a.wav") {
			println!("Verifying album artist");
			assert_eq!(tag.album_artists_vec(), Some(vec!["foo album artist"]));
		}
	};
}

macro_rules! remove_tags {
	($file:expr) => {
		println!("Reading file");
		let mut tag = Tag::default().read_from_path_signature($file).unwrap();

		println!("Removing title");
		tag.remove_title();
		assert!(tag.title().is_none());
		tag.remove_title(); // should not panic

		println!("Removing artist");
		tag.remove_artist();
		assert!(tag.artist_str().is_none());
		tag.remove_artist();

		println!("Removing year");
		tag.remove_year();
		assert!(tag.year().is_none());
		tag.remove_year();

		println!("Removing album title");
		tag.remove_album_title();
		assert!(tag.album_title().is_none());
		tag.remove_album_title();

		println!("Removing album artists");
		tag.remove_album_artists();
		assert!(tag.album_artists_vec().is_none());
		tag.remove_album_artists();

		tag.remove_album_covers();
		assert_eq!(tag.album_covers(), (None, None));
		tag.remove_album_covers();

		println!("Writing");
		tag.write_to_path($file).unwrap();
	};
}

// APEv2
full_test!(test_ape, "tests/assets/a.ape");

// ID3v2
full_test!(test_mp3, "tests/assets/a.mp3");
full_test!(test_aiff, "tests/assets/a.aiff");
full_test!(test_wav_id3, "tests/assets/a-id3.wav");

// RIFF INFO
full_test!(test_wav_riff_info, "tests/assets/a.wav");

// Vorbis comments
full_test!(test_flac, "tests/assets/a.flac");
full_test!(test_m4a, "tests/assets/a.m4a");
full_test!(test_ogg, "tests/assets/a.ogg");
full_test!(test_opus, "tests/assets/a.opus");
