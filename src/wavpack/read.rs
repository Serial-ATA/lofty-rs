use super::properties::WavPackProperties;
use super::WavPackFile;
use crate::ape::constants::APE_PREAMBLE;
use crate::ape::header::read_ape_header;
use crate::ape::tag::read::read_ape_tag;
use crate::error::Result;
use crate::id3::{find_id3v1, find_lyrics3v2, ID3FindResults};

use std::io::{Read, Seek, SeekFrom};

pub(super) fn read_from<R>(reader: &mut R, _read_properties: bool) -> Result<WavPackFile>
where
	R: Read + Seek,
{
	#[cfg(feature = "id3v1")]
	let mut id3v1_tag = None;
	#[cfg(feature = "ape")]
	let mut ape_tag = None;

	let ID3FindResults(id3v1_header, id3v1) = find_id3v1(reader, true)?;

	if id3v1_header.is_some() {
		#[cfg(feature = "id3v1")]
		{
			id3v1_tag = id3v1;
		}
	}

	// Next, check for a Lyrics3v2 tag, and skip over it, as it's no use to us
	let ID3FindResults(_lyrics3_header, _lyrics3v2_size) = find_lyrics3v2(reader)?;

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
		properties: WavPackProperties::default(),
	})
}
