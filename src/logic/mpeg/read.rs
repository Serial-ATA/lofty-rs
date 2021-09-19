use super::header::{verify_frame_sync, Header, XingHeader};
use super::MpegFile;
use crate::error::{LoftyError, Result};
use crate::logic::id3::unsynch_u32;
use crate::logic::id3::v2::read::parse_id3v2;
use crate::types::properties::FileProperties;

use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

use byteorder::{BigEndian, ByteOrder, ReadBytesExt};

fn read_properties(
	first_frame: (Header, u64),
	last_frame: (Header, u64),
	xing_header: Option<XingHeader>,
) -> FileProperties {
	let (duration, bitrate) = {
		if let Some(xing_header) = xing_header {
			if first_frame.0.samples > 0 && first_frame.0.sample_rate > 0 {
				let frame_time =
					u32::from(first_frame.0.samples) * 1000 / first_frame.0.sample_rate;
				let length = u64::from(frame_time) * u64::from(xing_header.frames);

				(
					Duration::from_millis(length),
					((u64::from(xing_header.size) * 8) / length) as u32,
				)
			} else {
				(Duration::ZERO, first_frame.0.bitrate)
			}
		} else if first_frame.0.bitrate > 0 {
			let bitrate = first_frame.0.bitrate;

			let stream_length = last_frame.1 - first_frame.1 + u64::from(first_frame.0.len);

			let length = if stream_length > 0 {
				Duration::from_millis((stream_length * 8) / u64::from(bitrate))
			} else {
				Duration::ZERO
			};

			(length, bitrate)
		} else {
			(Duration::ZERO, 0)
		}
	};

	FileProperties::new(
		duration,
		Some(bitrate),
		Some(first_frame.0.sample_rate),
		Some(first_frame.0.channels as u8),
	)
}

#[allow(clippy::similar_names)]
pub(crate) fn read_from<R>(data: &mut R) -> Result<MpegFile>
where
	R: Read + Seek,
{
	let mut mpeg_file = MpegFile {
		id3v2: None,
		id3v1: None,
		ape: None,
		properties: FileProperties::default(),
	};

	let mut first_mpeg_frame = (None, 0);
	let mut last_mpeg_frame = (None, 0);

	// Skip any invalid padding
	while data.read_u8()? == 0 {}

	data.seek(SeekFrom::Current(-1))?;

	let mut header = [0; 4];

	while let Ok(()) = data.read_exact(&mut header) {
		match header {
			_ if verify_frame_sync(u16::from_be_bytes([header[0], header[1]])) => {
				let start = data.seek(SeekFrom::Current(0))? - 4;
				let header = Header::read(u32::from_be_bytes(header))?;
				data.seek(SeekFrom::Current(i64::from(header.len - 4)))?;

				if first_mpeg_frame.0.is_none() {
					first_mpeg_frame = (Some(header), start);
				}

				last_mpeg_frame = (Some(header), start);
			},
			// [I, D, 3, ver_major, ver_minor, flags, size (4 bytes)]
			[b'I', b'D', b'3', ..] => {
				let mut remaining_header = [0; 6];
				data.read_exact(&mut remaining_header)?;

				let size = (unsynch_u32(BigEndian::read_u32(&remaining_header[2..])) + 10) as usize;
				data.seek(SeekFrom::Current(-10))?;

				let mut id3v2_read = vec![0; size];
				data.read_exact(&mut id3v2_read)?;

				let id3v2 = parse_id3v2(&mut &*id3v2_read)?;

				// Skip over the footer
				if id3v2.flags().footer {
					data.seek(SeekFrom::Current(10))?;
				}

				mpeg_file.id3v2 = Some(id3v2);

				continue;
			},
			[b'T', b'A', b'G', ..] => {
				data.seek(SeekFrom::Current(-4))?;

				let mut id3v1_read = [0; 128];
				data.read_exact(&mut id3v1_read)?;

				mpeg_file.id3v1 = Some(crate::logic::id3::v1::read::parse_id3v1(id3v1_read));
				continue;
			},
			[b'A', b'P', b'E', b'T'] => {
				let mut header_remaining = [0; 4];
				data.read_exact(&mut header_remaining)?;

				if &header_remaining == b"AGEX" {
					mpeg_file.ape =
						Some(crate::logic::ape::tag::read::read_ape_tag(data, false)?.0);
					continue;
				}
			},
			_ => return Err(LoftyError::Mp3("File contains an invalid frame")),
		}
	}

	if first_mpeg_frame.0.is_none() {
		return Err(LoftyError::Mp3("Unable to find an MPEG frame"));
	}

	let first_mpeg_frame = (first_mpeg_frame.0.unwrap(), first_mpeg_frame.1);
	let last_mpeg_frame = (last_mpeg_frame.0.unwrap(), last_mpeg_frame.1);

	let xing_header_location = first_mpeg_frame.1 + u64::from(first_mpeg_frame.0.data_start);

	data.seek(SeekFrom::Start(xing_header_location))?;

	let mut xing_reader = [0; 32];
	data.read_exact(&mut xing_reader)?;

	let xing_header = XingHeader::read(&mut &xing_reader[..]).ok();

	mpeg_file.properties = read_properties(first_mpeg_frame, last_mpeg_frame, xing_header);

	Ok(mpeg_file)
}
