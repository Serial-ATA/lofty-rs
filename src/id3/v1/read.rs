use super::constants::GENRES;
use super::tag::ID3v1Tag;

pub fn parse_id3v1(reader: [u8; 128]) -> ID3v1Tag {
	let mut tag = ID3v1Tag {
		title: None,
		artist: None,
		album: None,
		year: None,
		comment: None,
		track_number: None,
		genre: None,
	};

	let reader = &reader[3..];

	tag.title = decode_text(&reader[..30]);
	tag.artist = decode_text(&reader[30..60]);
	tag.album = decode_text(&reader[60..90]);
	tag.year = decode_text(&reader[90..94]);

	// Determine the range of the comment (30 bytes for ID3v1 and 28 for ID3v1.1)
	// We check for the null terminator 28 bytes in, and for a non-zero track number after it.
	// A track number of 0 is invalid.
	let range = if reader[122] == 0 && reader[123] != 0 {
		tag.track_number = Some(reader[123]);

		94_usize..123
	} else {
		94..124
	};

	tag.comment = decode_text(&reader[range]);

	if reader[124] < GENRES.len() as u8 {
		tag.genre = Some(reader[124]);
	}

	tag
}

fn decode_text(data: &[u8]) -> Option<String> {
	let read = data
		.iter()
		.filter(|c| **c != 0)
		.map(|c| *c as char)
		.collect::<String>();

	if read.is_empty() {
		None
	} else {
		Some(read)
	}
}
