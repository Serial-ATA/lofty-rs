use super::header::{search_for_frame_sync, Header, XingHeader};
use super::{Mp3File, Mp3Properties};
use crate::ape::constants::APE_PREAMBLE;
use crate::ape::header::read_ape_header;
#[cfg(feature = "ape")]
use crate::ape::tag::read::read_ape_tag;
use crate::error::{FileDecodingError, Result};
use crate::file::FileType;
#[cfg(feature = "id3v2")]
use crate::id3::v2::read::parse_id3v2;
use crate::id3::v2::read_id3v2_header;
use crate::id3::{find_id3v1, find_lyrics3v2, ID3FindResults};

use std::io::{Read, Seek, SeekFrom};

use byteorder::{BigEndian, ReadBytesExt};

pub(super) fn read_from<R>(
	reader: &mut R,
	read_tags: bool,
	read_properties: bool,
) -> Result<Mp3File>
where
	R: Read + Seek,
{
	let mut file = Mp3File::default();

	let mut first_frame_header = None;

	// Skip any invalid padding
	while reader.read_u8()? == 0 {}

	reader.seek(SeekFrom::Current(-1))?;

	let mut header = [0; 4];

	while let Ok(()) = reader.read_exact(&mut header) {
		match header {
			// [I, D, 3, ver_major, ver_minor, flags, size (4 bytes)]
			[b'I', b'D', b'3', ..] => {
				let mut remaining_header = [0; 6];
				reader.read_exact(&mut remaining_header)?;

				let header = read_id3v2_header(
					&mut &*[header.as_slice(), remaining_header.as_slice()].concat(),
				)?;
				let skip_footer = header.flags.footer;

				#[cfg(feature = "id3v2")]
				if read_tags {
					let id3v2 = parse_id3v2(reader, header)?;
					file.id3v2_tag = Some(id3v2);
				}

				// Skip over the footer
				if skip_footer {
					reader.seek(SeekFrom::Current(10))?;
				}

				continue;
			},
			[b'A', b'P', b'E', b'T'] => {
				let mut header_remaining = [0; 4];
				reader.read_exact(&mut header_remaining)?;

				if &header_remaining == b"AGEX" {
					let ape_header = read_ape_header(reader, false)?;

					#[cfg(not(feature = "ape"))]
					{
						let size = ape_header.size;
						reader.seek(SeekFrom::Current(size as i64))?;
					}

					#[cfg(feature = "ape")]
					if read_tags {
						file.ape_tag =
							Some(crate::ape::tag::read::read_ape_tag(reader, ape_header)?);
					}

					continue;
				}
			},
			// Tags might be followed by junk bytes before the first MP3 frame begins
			_ => {
				// Seek back the length of the temporary header buffer, to include them
				// in the frame sync search
				#[allow(clippy::neg_multiply)]
				let start_of_search_area = reader.seek(SeekFrom::Current(-1 * header.len() as i64))?;

				if let Some(first_mp3_frame_start_relative) = search_for_frame_sync(reader)? {
					let first_mp3_frame_start_absolute =
						start_of_search_area + first_mp3_frame_start_relative;

					// Seek back to the start of the frame and read the header
					reader.seek(SeekFrom::Start(first_mp3_frame_start_absolute))?;
					let header = Header::read(reader.read_u32::<BigEndian>()?)?;

					file.first_frame_offset = first_mp3_frame_start_absolute;
					first_frame_header = Some(header);

					// We have found the first frame
					break;
				}

				// The search for sync bits was unsuccessful
				return Err(FileDecodingError::new(
					FileType::MP3,
					"File contains an invalid frame",
				)
				.into());
			},
		}
	}

	#[allow(unused_variables)]
	let ID3FindResults(header, id3v1) = find_id3v1(reader, true)?;

	#[cfg(feature = "id3v1")]
	if header.is_some() {
		file.id3v1_tag = id3v1;
	}

	let _ = find_lyrics3v2(reader)?;

	reader.seek(SeekFrom::Current(-32))?;

	let mut ape_preamble = [0; 8];
	reader.read_exact(&mut ape_preamble)?;

	if &ape_preamble == APE_PREAMBLE {
		let ape_header = read_ape_header(reader, true)?;
		let size = ape_header.size;

		#[cfg(feature = "ape")]
		if read_tags {
			let ape = read_ape_tag(reader, ape_header)?;
			file.ape_tag = Some(ape);
		}

		// Seek back to the start of the tag
		let pos = reader.seek(SeekFrom::Current(0))?;
		reader.seek(SeekFrom::Start(pos - u64::from(size)))?;
	}

	file.last_frame_offset = reader.seek(SeekFrom::Current(0))?;

	file.properties = if read_properties {
		// Safe to unwrap, since we return early if no frame is found
		let first_frame_header = first_frame_header.unwrap();

		if first_frame_header.sample_rate == 0 {
			return Err(FileDecodingError::new(FileType::MP3, "Sample rate is 0").into());
		}

		let first_frame_offset = file.first_frame_offset;

		let file_length = reader.seek(SeekFrom::End(0))?;

		let xing_header_location = first_frame_offset + u64::from(first_frame_header.data_start);

		reader.seek(SeekFrom::Start(xing_header_location))?;

		let mut xing_reader = [0; 32];
		reader.read_exact(&mut xing_reader)?;

		let xing_header = XingHeader::read(&mut &xing_reader[..])?;

		super::properties::read_properties(
			(first_frame_header, first_frame_offset),
			file.last_frame_offset,
			xing_header,
			file_length,
		)
	} else {
		Mp3Properties::default()
	};

	Ok(file)
}
