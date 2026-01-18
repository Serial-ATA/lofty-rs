use super::header::{Header, HeaderCmpResult, VbrHeader, cmp_header, search_for_frame_sync};
use super::{MpegFile, MpegProperties};
use crate::ape::header::read_ape_header;
use crate::config::{ParseOptions, ParsingMode};
use crate::error::Result;
use crate::id3::v2::header::Id3v2Header;
use crate::id3::v2::read::parse_id3v2;
use crate::id3::{FindId3v2Config, ID3FindResults, find_id3v1, find_lyrics3v2};
use crate::io::SeekStreamLen;
use crate::macros::{decode_err, err};
use crate::mpeg::header::HEADER_MASK;

use std::io::{Read, Seek, SeekFrom};

use byteorder::{BigEndian, ReadBytesExt};

pub(super) fn read_from<R>(reader: &mut R, parse_options: ParseOptions) -> Result<MpegFile>
where
	R: Read + Seek,
{
	let mut file = MpegFile::default();

	let mut first_frame_offset = 0;
	let mut first_frame_header = None;

	// Skip any invalid padding
	while reader.read_u8()? == 0 {}

	reader.seek(SeekFrom::Current(-1))?;

	let mut header = [0; 4];

	while let Ok(()) = reader.read_exact(&mut header) {
		match header {
			// [I, D, 3, ver_major, ver_minor, flags, size (4 bytes)]
			//
			// Best case scenario, we find an ID3v2 tag at the beginning of the file.
			// We will check again after finding the frame sync, in case the tag is buried in junk.
			[b'I', b'D', b'3', ..] => {
				// Seek back to read the tag in full
				reader.seek(SeekFrom::Current(-4))?;

				let header = Id3v2Header::parse(reader)?;
				let skip_footer = header.flags.footer;

				if parse_options.read_tags {
					let id3v2 = parse_id3v2(reader, header, parse_options)?;
					if let Some(existing_tag) = &mut file.id3v2_tag {
						// https://github.com/Serial-ATA/lofty-rs/issues/87
						// Duplicate tags should have their frames appended to the previous
						for frame in id3v2.frames {
							existing_tag.insert(frame);
						}
						continue;
					}
					file.id3v2_tag = Some(id3v2);
				} else {
					reader.seek(SeekFrom::Current(i64::from(header.size)))?;
				}

				// Skip over the footer
				if skip_footer {
					reader.seek(SeekFrom::Current(10))?;
				}

				continue;
			},
			// TODO: APE tags may suffer the same issue as ID3v2 tag described above.
			//       They are not nearly as important to preserve, however.
			[b'A', b'P', b'E', b'T'] => {
				log::warn!(
					"Encountered an APE tag at the beginning of the file, attempting to read"
				);

				let mut header_remaining = [0; 4];
				reader.read_exact(&mut header_remaining)?;

				if &header_remaining == b"AGEX" {
					let ape_header = read_ape_header(reader, false)?;

					if parse_options.read_tags {
						file.ape_tag = Some(crate::ape::tag::read::read_ape_tag_with_header(
							reader,
							ape_header,
							parse_options,
						)?);
					} else {
						reader.seek(SeekFrom::Current(i64::from(ape_header.size)))?;
					}

					continue;
				}

				err!(FakeTag);
			},
			// Tags might be followed by junk bytes before the first MP3 frame begins
			_ => {
				// Seek back the length of the temporary header buffer, to include them
				// in the frame sync search
				#[allow(clippy::neg_multiply)]
				reader.seek(SeekFrom::Current(-1 * header.len() as i64))?;

				let Some((_first_frame_header, _first_frame_offset)) = find_next_frame(reader)?
				else {
					break;
				};

				if file.id3v2_tag.is_none()
					&& parse_options.parsing_mode != ParsingMode::Strict
					&& _first_frame_offset > 0
				{
					reader.seek(SeekFrom::Start(0))?;

					let search_window_size =
						std::cmp::min(_first_frame_offset, parse_options.max_junk_bytes as u64);

					let config = FindId3v2Config {
						read: parse_options.read_tags,
						allowed_junk_window: Some(search_window_size),
					};

					if let ID3FindResults(Some(header), Some(id3v2_bytes)) =
						crate::id3::find_id3v2(reader, config)?
					{
						let reader = &mut &*id3v2_bytes;

						let id3v2 = parse_id3v2(reader, header, parse_options)?;

						if let Some(existing_tag) = &mut file.id3v2_tag {
							// https://github.com/Serial-ATA/lofty-rs/issues/87
							// Duplicate tags should have their frames appended to the previous
							for frame in id3v2.frames {
								existing_tag.insert(frame);
							}
							continue;
						}

						file.id3v2_tag = Some(id3v2);
					}
				}

				first_frame_offset = _first_frame_offset;
				first_frame_header = Some(_first_frame_header);
				break;
			},
		}
	}

	#[allow(unused_variables)]
	let ID3FindResults(header, id3v1) =
		find_id3v1(reader, parse_options.read_tags, parse_options.parsing_mode)?;

	if header.is_some() {
		file.id3v1_tag = id3v1;
	}

	let _ = find_lyrics3v2(reader)?;

	reader.seek(SeekFrom::Current(-32))?;

	match crate::ape::tag::read::read_ape_tag(reader, true, parse_options)? {
		(tag, Some(header)) => {
			file.ape_tag = tag;

			// Seek back to the start of the tag
			let pos = reader.stream_position()?;
			let Some(start_of_tag) = pos.checked_sub(u64::from(header.size)) else {
				err!(SizeMismatch);
			};

			reader.seek(SeekFrom::Start(start_of_tag))?;
		},
		_ => {
			// Correct the position (APE header - Preamble)
			reader.seek(SeekFrom::Current(24))?;
		},
	}

	let last_frame_offset = reader.stream_position()?;
	file.properties = MpegProperties::default();

	if parse_options.read_properties {
		let Some(first_frame_header) = first_frame_header else {
			// The search for sync bits was unsuccessful
			decode_err!(@BAIL Mpeg, "File contains an invalid frame");
		};

		if first_frame_header.sample_rate == 0 {
			decode_err!(@BAIL Mpeg, "Sample rate is 0");
		}

		let first_frame_offset = first_frame_offset;

		// Try to read a Xing header
		let xing_header_location = first_frame_offset + u64::from(first_frame_header.data_start);
		reader.seek(SeekFrom::Start(xing_header_location))?;

		let mut xing_reader = [0; 32];
		reader.read_exact(&mut xing_reader)?;

		let xing_header = VbrHeader::read(&mut &xing_reader[..])?;

		let file_length = reader.stream_len_hack()?;

		super::properties::read_properties(
			&mut file.properties,
			reader,
			(first_frame_header, first_frame_offset),
			last_frame_offset,
			xing_header,
			file_length,
		)?;
	}

	Ok(file)
}

// Searches for the next frame, comparing it to the following one
fn find_next_frame<R>(reader: &mut R) -> Result<Option<(Header, u64)>>
where
	R: Read + Seek,
{
	let mut pos = reader.stream_position()?;

	while let Ok(Some(first_mp3_frame_start_relative)) = search_for_frame_sync(reader) {
		let first_mp3_frame_start_absolute = pos + first_mp3_frame_start_relative;

		// Seek back to the start of the frame and read the header
		reader.seek(SeekFrom::Start(first_mp3_frame_start_absolute))?;
		let first_header_data = reader.read_u32::<BigEndian>()?;

		if let Some(first_header) = Header::read(first_header_data) {
			match cmp_header(reader, 4, first_header.len, first_header_data, HEADER_MASK) {
				HeaderCmpResult::Equal => {
					return Ok(Some((first_header, first_mp3_frame_start_absolute)));
				},
				HeaderCmpResult::Undetermined => return Ok(None),
				HeaderCmpResult::NotEqual => {},
			}
		}

		pos = reader.stream_position()?;
	}

	Ok(None)
}
