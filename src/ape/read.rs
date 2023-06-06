use super::header::read_ape_header;
use super::tag::ApeTag;
use super::{ApeFile, ApeProperties};
use crate::ape::tag::read::{read_ape_tag, read_ape_tag_with_header};
use crate::error::Result;
use crate::id3::v1::tag::Id3v1Tag;
use crate::id3::v2::read::parse_id3v2;
use crate::id3::v2::tag::Id3v2Tag;
use crate::id3::{find_id3v1, find_id3v2, find_lyrics3v2, ID3FindResults};
use crate::macros::decode_err;
use crate::probe::ParseOptions;

use std::io::{Read, Seek, SeekFrom};

pub(crate) fn read_from<R>(data: &mut R, parse_options: ParseOptions) -> Result<ApeFile>
where
	R: Read + Seek,
{
	let start = data.stream_position()?;
	let end = data.seek(SeekFrom::End(0))?;

	data.seek(SeekFrom::Start(start))?;

	let mut stream_len = end - start;

	let mut id3v2_tag: Option<Id3v2Tag> = None;
	let mut id3v1_tag: Option<Id3v1Tag> = None;
	let mut ape_tag: Option<ApeTag> = None;

	// ID3v2 tags are unsupported in APE files, but still possible
	#[allow(unused_variables)]
	if let ID3FindResults(Some(header), Some(content)) = find_id3v2(data, true)? {
		stream_len -= u64::from(header.size);

		// Exclude the footer
		if header.flags.footer {
			stream_len -= 10;
		}

		let reader = &mut &*content;

		let id3v2 = parse_id3v2(reader, header, parse_options.parsing_mode)?;
		id3v2_tag = Some(id3v2);
	}

	let mut found_mac = false;
	let mut mac_start = 0;

	let mut header = [0; 4];
	data.read_exact(&mut header)?;

	while !found_mac {
		match &header {
			b"MAC " => {
				mac_start = data.stream_position()?;

				found_mac = true;
			},
			// An APE tag at the beginning of the file goes against the spec, but is still possible.
			// This only allows for v2 tags though, since it relies on the header.
			b"APET" => {
				// Get the remaining part of the ape tag
				let mut remaining = [0; 4];
				data.read_exact(&mut remaining).map_err(|_| {
					decode_err!(
						Ape,
						"Found partial APE tag, but there isn't enough data left in the reader"
					)
				})?;

				if &remaining[..4] != b"AGEX" {
					decode_err!(@BAIL Ape, "Found incomplete APE tag");
				}

				let ape_header = read_ape_header(data, false)?;
				stream_len -= u64::from(ape_header.size);

				let ape = read_ape_tag_with_header(data, ape_header)?;
				ape_tag = Some(ape);
			},
			_ => {
				decode_err!(@BAIL Ape, "Invalid data found while reading header, expected any of [\"MAC \", \"APETAGEX\", \"ID3\"]")
			},
		}
	}

	// First see if there's a ID3v1 tag
	//
	// Starts with ['T', 'A', 'G']
	// Exactly 128 bytes long (including the identifier)
	#[allow(unused_variables)]
	let ID3FindResults(id3v1_header, id3v1) = find_id3v1(data, true)?;

	if id3v1_header.is_some() {
		stream_len -= 128;
		id3v1_tag = id3v1;
	}

	// Next, check for a Lyrics3v2 tag, and skip over it, as it's no use to us
	let ID3FindResults(lyrics3_header, lyrics3v2_size) = find_lyrics3v2(data)?;

	if lyrics3_header.is_some() {
		stream_len -= u64::from(lyrics3v2_size)
	}

	// Next, search for an APE tag footer
	//
	// Starts with ['A', 'P', 'E', 'T', 'A', 'G', 'E', 'X']
	// Exactly 32 bytes long
	// Strongly recommended to be at the end of the file
	data.seek(SeekFrom::Current(-32))?;

	if let Some((tag, header)) = read_ape_tag(data, true)? {
		stream_len -= u64::from(header.size);
		ape_tag = Some(tag);
	}

	let file_length = data.stream_position()?;

	// Go back to the MAC header to read properties
	data.seek(SeekFrom::Start(mac_start))?;

	Ok(ApeFile {
		id3v1_tag,
		id3v2_tag,
		ape_tag,
		properties: if parse_options.read_properties {
			super::properties::read_properties(data, stream_len, file_length)?
		} else {
			ApeProperties::default()
		},
	})
}
