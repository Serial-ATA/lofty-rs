use super::WavPackFile;
use super::properties::WavPackProperties;
use crate::config::ParseOptions;
use crate::error::Result;
use crate::id3::{ID3FindResults, find_id3v1, find_lyrics3v2};

use crate::macros::err;
use std::io::{Read, Seek, SeekFrom};

pub(super) fn read_from<R>(reader: &mut R, parse_options: ParseOptions) -> Result<WavPackFile>
where
	R: Read + Seek,
{
	let current_pos = reader.stream_position()?;
	let mut stream_length = reader.seek(SeekFrom::End(0))?;
	reader.seek(SeekFrom::Start(current_pos))?;

	let mut id3v1_tag = None;
	let mut ape_tag = None;

	let ID3FindResults(id3v1_header, id3v1) =
		find_id3v1(reader, parse_options.read_tags, parse_options.parsing_mode)?;

	if id3v1_header.is_some() {
		id3v1_tag = id3v1;
		let Some(new_stream_length) = stream_length.checked_sub(128) else {
			err!(SizeMismatch);
		};

		stream_length = new_stream_length;
	}

	// Next, check for a Lyrics3v2 tag, and skip over it, as it's no use to us
	let ID3FindResults(_, lyrics3v2_size) = find_lyrics3v2(reader)?;
	let Some(new_stream_length) = stream_length.checked_sub(u64::from(lyrics3v2_size)) else {
		err!(SizeMismatch);
	};

	stream_length = new_stream_length;

	// Next, search for an APE tag footer
	//
	// Starts with ['A', 'P', 'E', 'T', 'A', 'G', 'E', 'X']
	// Exactly 32 bytes long
	// Strongly recommended to be at the end of the file
	reader.seek(SeekFrom::Current(-32))?;

	if let (tag, Some(header)) = crate::ape::tag::read::read_ape_tag(reader, true, parse_options)? {
		stream_length -= u64::from(header.size);
		ape_tag = tag;
	}

	Ok(WavPackFile {
		id3v1_tag,
		ape_tag,
		properties: if parse_options.read_properties {
			super::properties::read_properties(reader, stream_length, parse_options.parsing_mode)?
		} else {
			WavPackProperties::default()
		},
	})
}
