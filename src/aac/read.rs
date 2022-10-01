use super::header::{ADTSHeader, HEADER_MASK};
use super::AACFile;
use crate::error::Result;
use crate::id3::v2::read::parse_id3v2;
use crate::id3::v2::read_id3v2_header;
use crate::id3::{find_id3v1, ID3FindResults};
use crate::macros::{decode_err, parse_mode_choice};
use crate::mpeg::header::{cmp_header, search_for_frame_sync, HeaderCmpResult};
use crate::probe::{ParseOptions, ParsingMode};

use std::io::{Read, Seek, SeekFrom};

use byteorder::ReadBytesExt;

#[allow(clippy::unnecessary_wraps)]
pub(super) fn read_from<R>(reader: &mut R, parse_options: ParseOptions) -> Result<AACFile>
where
	R: Read + Seek,
{
	let parse_mode = parse_options.parsing_mode;

	let mut file = AACFile::default();

	let mut first_frame_header = None;
	let mut first_frame_end = 0;

	// Skip any invalid padding
	while reader.read_u8()? == 0 {}

	reader.seek(SeekFrom::Current(-1))?;

	let pos = reader.stream_position()?;
	let mut stream_len = reader.seek(SeekFrom::End(0))?;

	reader.seek(SeekFrom::Start(pos))?;

	let mut header = [0; 4];

	while let Ok(()) = reader.read_exact(&mut header) {
		match header {
			// [I, D, 3, ver_major, ver_minor, flags, size (4 bytes)]
			[b'I', b'D', b'3', ..] => {
				// Seek back to read the tag in full
				reader.seek(SeekFrom::Current(-4))?;

				let header = read_id3v2_header(reader)?;
				let skip_footer = header.flags.footer;

				stream_len -= u64::from(header.size);

				#[cfg(feature = "id3v2")]
				{
					let id3v2 = parse_id3v2(reader, header)?;
					file.id3v2_tag = Some(id3v2);
				}

				// Skip over the footer
				if skip_footer {
					stream_len -= 10;
					reader.seek(SeekFrom::Current(10))?;
				}

				continue;
			},
			// Tags might be followed by junk bytes before the first ADTS frame begins
			_ => {
				// Seek back the length of the temporary header buffer, to include them
				// in the frame sync search
				#[allow(clippy::neg_multiply)]
				reader.seek(SeekFrom::Current(-1 * header.len() as i64))?;

				if let Some((first_frame_header_, first_frame_end_)) =
					find_next_frame(reader, parse_mode)?
				{
					first_frame_header = Some(first_frame_header_);
					first_frame_end = first_frame_end_;
					break;
				}
			},
		}
	}

	#[allow(unused_variables)]
	let ID3FindResults(header, id3v1) = find_id3v1(reader, true)?;

	#[cfg(feature = "id3v1")]
	if header.is_some() {
		stream_len -= 128;
		file.id3v1_tag = id3v1;
	}

	if parse_options.read_properties {
		let mut first_frame_header = match first_frame_header {
			Some(header) => header,
			// The search for sync bits was unsuccessful
			None => decode_err!(@BAIL MPEG, "File contains an invalid frame"),
		};

		if first_frame_header.sample_rate == 0 {
			parse_mode_choice!(
				parse_mode,
				STRICT: decode_err!(@BAIL MPEG, "Sample rate is 0"),
			);
		}

		if first_frame_header.bitrate == 0 {
			parse_mode_choice!(parse_mode, STRICT: decode_err!(@BAIL MPEG, "Bitrate is 0"),);
		}

		// Read as many frames as we can to try and fine the average bitrate
		reader.seek(SeekFrom::Start(first_frame_end))?;

		let mut frame_count = 1;

		while let Some((header, _)) = find_next_frame(reader, parse_mode)? {
			first_frame_header.bitrate += header.bitrate;
			frame_count += 1u32;
		}

		first_frame_header.bitrate /= frame_count;

		super::properties::read_properties(&mut file.properties, first_frame_header, stream_len);
	}

	Ok(file)
}

// Searches for the next frame, comparing it to the following one
fn find_next_frame<R>(
	reader: &mut R,
	parsing_mode: ParsingMode,
) -> Result<Option<(ADTSHeader, u64)>>
where
	R: Read + Seek,
{
	let mut pos = reader.stream_position()?;

	while let Ok(Some(first_adts_frame_start_relative)) = search_for_frame_sync(reader) {
		let first_adts_frame_start_absolute = pos + first_adts_frame_start_relative;

		// Seek back to the start of the frame and read the header
		reader.seek(SeekFrom::Start(first_adts_frame_start_absolute))?;

		if let Some(first_header) = ADTSHeader::read(reader, parsing_mode)? {
			let header_len = if first_header.has_crc { 9 } else { 7 };

			match cmp_header(
				reader,
				header_len,
				u32::from(first_header.len),
				u32::from_be_bytes(first_header.bytes[..4].try_into().unwrap()),
				HEADER_MASK,
			) {
				HeaderCmpResult::Equal => {
					return Ok(Some((
						first_header,
						first_adts_frame_start_absolute + u64::from(header_len),
					)))
				},
				HeaderCmpResult::Undetermined => return Ok(None),
				HeaderCmpResult::NotEqual => {},
			}
		}

		pos = reader.stream_position()?;
	}

	Ok(None)
}
