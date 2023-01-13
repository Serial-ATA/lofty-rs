use crate::temp_file;

use std::io::Seek;

use lofty::id3::v1::{ID3v1Tag, GENRES};
use lofty::mpeg::MPEGFile;
use lofty::{Accessor, AudioFile, ParseOptions};

#[test]
#[ignore] // TODO: We probably should be stripping whitespace
fn test_strip_whitespace() {
	let mut file = temp_file!("tests/taglib/data/xing.mp3");
	{
		let mut f = MPEGFile::read_from(&mut file, ParseOptions::new()).unwrap();
		file.rewind().unwrap();

		let mut tag = ID3v1Tag::default();
		tag.set_artist(String::from("Artist     "));
		f.set_id3v1(tag);
		f.save_to(&mut file).unwrap();
	}
	file.rewind().unwrap();
	{
		let f = MPEGFile::read_from(&mut file, ParseOptions::new()).unwrap();
		assert_eq!(f.id3v1().unwrap().artist().as_deref(), Some("Artist"));
	}
}

#[test]
fn test_genres() {
	assert_eq!("Darkwave", GENRES[50]);
	assert_eq!(
		100,
		GENRES.iter().position(|genre| *genre == "Humour").unwrap()
	);
	assert!(GENRES.contains(&"Heavy Metal"));
	assert_eq!(
		79,
		GENRES
			.iter()
			.position(|genre| *genre == "Hard Rock")
			.unwrap()
	);
}

#[test]
#[ignore]
fn test_renamed_genres() {
	// Marker test, this covers a change where TagLib deviated from the list of genres available on Wikipedia.
	// For now, Lofty has no reason to change.
}
