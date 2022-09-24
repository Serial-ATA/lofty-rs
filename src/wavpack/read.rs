use super::properties::WavPackProperties;
use super::WavPackFile;
use crate::ape::constants::APE_PREAMBLE;
use crate::ape::header::read_ape_header;
use crate::ape::tag::read::read_ape_tag;
use crate::error::Result;
use crate::id3::{find_id3v1, find_lyrics3v2, ID3FindResults};
use crate::probe::ParseOptions;

use std::io::{Read, Seek, SeekFrom};

pub(super) fn read_from<R>(reader: &mut R, parse_options: ParseOptions) -> Result<WavPackFile>
where
	R: Read + Seek,
{
	let current_pos = reader.stream_position()?;
	let mut stream_length = reader.seek(SeekFrom::End(0))?;
	reader.seek(SeekFrom::Start(current_pos))?;

	#[cfg(feature = "id3v1")]
	let mut id3v1_tag = None;
	#[cfg(feature = "ape")]
	let mut ape_tag = None;

	let ID3FindResults(id3v1_header, id3v1) = find_id3v1(reader, true)?;

	if id3v1_header.is_some() {
		stream_length -= 128;
		#[cfg(feature = "id3v1")]
		{
			id3v1_tag = id3v1;
		}
	}

	// Next, check for a Lyrics3v2 tag, and skip over it, as it's no use to us
	let ID3FindResults(lyrics3_header, lyrics3v2_size) = find_lyrics3v2(reader)?;

	if lyrics3_header.is_some() {
		stream_length -= u64::from(lyrics3v2_size);
	}

	// Next, search for an APE tag footer
	//
	// Starts with ['A', 'P', 'E', 'T', 'A', 'G', 'E', 'X']
	// Exactly 32 bytes long
	// Strongly recommended to be at the end of the file
	reader.seek(SeekFrom::Current(-32))?;

	let mut ape_preamble = [0; 8];
	reader.read_exact(&mut ape_preamble)?;

	if &ape_preamble == APE_PREAMBLE {
		let ape_header = read_ape_header(reader, true)?;
		stream_length -= u64::from(ape_header.size);

		#[cfg(feature = "ape")]
		{
			let ape = read_ape_tag(reader, ape_header)?;
			ape_tag = Some(ape)
		}

		#[cfg(not(feature = "ape"))]
		data.seek(SeekFrom::Current(ape_header.size as i64))?;
	}

	Ok(WavPackFile {
		#[cfg(feature = "id3v1")]
		id3v1_tag,
		#[cfg(feature = "ape")]
		ape_tag,
		properties: if parse_options.read_properties {
			super::properties::read_properties(reader, stream_length, parse_options.parsing_mode)?
		} else {
			WavPackProperties::default()
		},
	})
}
