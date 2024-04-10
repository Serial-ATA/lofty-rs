use lofty::flac::FlacFile;
use lofty::prelude::*;
use lofty::{ParseOptions, ParsingMode};

use std::fs::File;
use std::io::Seek;

#[test]
fn multiple_vorbis_comments() {
	let mut file = File::open("tests/files/assets/two_vorbis_comments.flac").unwrap();

	// Reading a file with multiple VORBIS_COMMENT blocks should error when using `Strict`, as it is
	// not allowed by spec.
	assert!(FlacFile::read_from(
		&mut file,
		ParseOptions::new()
			.read_properties(false)
			.parsing_mode(ParsingMode::Strict)
	)
	.is_err());

	file.rewind().unwrap();

	// But by default, we should just take the last tag in the stream
	let f = FlacFile::read_from(&mut file, ParseOptions::new().read_properties(false)).unwrap();

	// The first tag has the artist "Artist 1", the second has "Artist 2".
	assert_eq!(
		f.vorbis_comments().unwrap().artist().as_deref(),
		Some("Artist 2")
	);
}
