use lofty::id3::v1::GENRES;

#[test]
#[ignore]
fn test_strip_whitespace() {
	// Marker test, we'd be overstepping to remove trailing whitespace that may be intentional
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
