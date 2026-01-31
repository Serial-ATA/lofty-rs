use super::{DffCommentRef, DffEditedMasterInfoRef};

/// Serialize DIIN chunk to bytes
pub(super) fn dump_diin_to_vec(diin: Option<DffEditedMasterInfoRef<'_>>) -> Vec<u8> {
	let mut diin_contents = Vec::new();

	if let Some(diin) = diin {
		// Write DIAR chunk (artist)
		if let Some(artist) = diin.artist {
			let mut artist_bytes = artist.as_bytes().to_vec();
			artist_bytes.push(0); // Null terminator

			diin_contents.extend_from_slice(b"DIAR");
			diin_contents.extend_from_slice(&(artist_bytes.len() as u64).to_be_bytes());
			diin_contents.extend_from_slice(&artist_bytes);
		}

		// Write DITI chunk (title)
		if let Some(title) = diin.title {
			let mut title_bytes = title.as_bytes().to_vec();
			title_bytes.push(0); // Null terminator

			diin_contents.extend_from_slice(b"DITI");
			diin_contents.extend_from_slice(&(title_bytes.len() as u64).to_be_bytes());
			diin_contents.extend_from_slice(&title_bytes);
		}
	}

	if diin_contents.is_empty() {
		return Vec::new();
	}

	// Wrap in DIIN container chunk
	let mut result = Vec::new();
	result.extend_from_slice(b"DIIN");
	result.extend_from_slice(&(diin_contents.len() as u64).to_be_bytes());
	result.extend_from_slice(&diin_contents);

	result
}

/// Serialize COMT chunk to bytes
pub(super) fn dump_comt_to_vec<'a>(comments: impl IntoIterator<Item = DffCommentRef<'a>>) -> Vec<u8> {
	use byteorder::{BigEndian, WriteBytesExt};

	let mut comt_contents = Vec::new();
	let mut count = 0u16;

	// Collect comments into a temporary buffer to count them
	let mut comment_data = Vec::new();
	for comment in comments {
		count += 1;

		// Timestamp (6 bytes) - use placeholder values
		comment_data.write_u16::<BigEndian>(2024).unwrap(); // year
		comment_data.write_u8(1).unwrap(); // month
		comment_data.write_u8(1).unwrap(); // day
		comment_data.write_u8(0).unwrap(); // hour
		comment_data.write_u8(0).unwrap(); // minutes

		// cmtType (2 bytes) - 0 = general comment
		comment_data.write_u16::<BigEndian>(0).unwrap();

		// cmtRef (2 bytes) - 0 = no reference
		comment_data.write_u16::<BigEndian>(0).unwrap();

		// Comment text
		let mut text_bytes = comment.text.as_bytes().to_vec();
		text_bytes.push(0); // Null terminator

		// count (4 bytes)
		comment_data
			.write_u32::<BigEndian>(text_bytes.len() as u32)
			.unwrap();
		comment_data.extend_from_slice(&text_bytes);
	}

	if count == 0 {
		return Vec::new();
	}

	// Number of comments (2 bytes)
	comt_contents
		.write_u16::<BigEndian>(count)
		.unwrap();
	comt_contents.extend_from_slice(&comment_data);

	// Wrap in COMT chunk
	let mut result = Vec::new();
	result.extend_from_slice(b"COMT");
	result.extend_from_slice(&(comt_contents.len() as u64).to_be_bytes());
	result.extend_from_slice(&comt_contents);

	result
}
