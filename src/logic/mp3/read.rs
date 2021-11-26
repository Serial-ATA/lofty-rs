use super::header::{verify_frame_sync, Header, XingHeader};
use super::{Mp3File, Mp3Properties};
use crate::error::{LoftyError, Result};
use crate::id3::v2::Id3v2Tag;
use crate::logic::ape::tag::ApeTag;
use crate::logic::id3::unsynch_u32;
use crate::logic::id3::v1::tag::Id3v1Tag;
use crate::logic::id3::v2::read::parse_id3v2;

use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

use byteorder::{BigEndian, ByteOrder, ReadBytesExt};

fn read_properties(
	mut first_frame: (Header, u64),
	last_frame: (Header, u64),
	xing_header: Option<XingHeader>,
	file_length: u64,
) -> Mp3Properties {
	let (duration, overall_bitrate, audio_bitrate) = {
		match xing_header {
			Some(xing_header) if first_frame.0.sample_rate > 0 => {
				let frame_time =
					u32::from(first_frame.0.samples) * 1000 / first_frame.0.sample_rate;
				let length = u64::from(frame_time) * u64::from(xing_header.frames);

				let overall_bitrate = ((file_length * 8) / length) as u32;
				let audio_bitrate = ((u64::from(xing_header.size) * 8) / length) as u32;

				(
					Duration::from_millis(length),
					overall_bitrate,
					audio_bitrate,
				)
			},
			_ if first_frame.0.bitrate > 0 => {
				let audio_bitrate = first_frame.0.bitrate;

				let stream_length = last_frame.1 - first_frame.1 + u64::from(first_frame.0.len);
				let length = (stream_length * 8) / u64::from(audio_bitrate);

				let overall_bitrate = ((file_length * 8) / length) as u32;

				let duration = Duration::from_millis(length);

				(duration, overall_bitrate, audio_bitrate)
			},
			_ => (Duration::ZERO, 0, 0),
		}
	};

	Mp3Properties {
		version: first_frame.0.version,
		layer: first_frame.0.layer,
		channel_mode: first_frame.0.channel_mode,
		duration,
		overall_bitrate,
		audio_bitrate,
		sample_rate: first_frame.0.sample_rate,
		channels: first_frame.0.channels as u8,
	}
}

#[allow(clippy::similar_names)]
pub(crate) fn read_from<R>(data: &mut R) -> Result<Mp3File>
where
	R: Read + Seek,
{
	let mut id3v2_tag: Option<Id3v2Tag> = None;
	let mut id3v1_tag: Option<Id3v1Tag> = None;
	let mut ape_tag: Option<ApeTag> = None;

	let mut first_mpeg_frame = (None, 0);
	let mut last_mpeg_frame = (None, 0);

	// Skip any invalid padding
	while data.read_u8()? == 0 {}

	data.seek(SeekFrom::Current(-1))?;

	let mut header = [0; 4];

	while let Ok(()) = data.read_exact(&mut header) {
		match header {
			_ if verify_frame_sync([header[0], header[1]]) => {
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

				id3v2_tag = Some(id3v2);

				continue;
			},
			[b'T', b'A', b'G', ..] => {
				data.seek(SeekFrom::Current(-4))?;

				let mut id3v1_read = [0; 128];
				data.read_exact(&mut id3v1_read)?;

				id3v1_tag = Some(crate::logic::id3::v1::read::parse_id3v1(id3v1_read));
				continue;
			},
			[b'A', b'P', b'E', b'T'] => {
				let mut header_remaining = [0; 4];
				data.read_exact(&mut header_remaining)?;

				if &header_remaining == b"AGEX" {
					ape_tag = Some(crate::logic::ape::tag::read::read_ape_tag(data, false)?.0);
					continue;
				}
			},
			_ => return Err(LoftyError::Mp3("File contains an invalid frame")),
		}
	}

	if first_mpeg_frame.0.is_none() {
		return Err(LoftyError::Mp3("Unable to find an MPEG frame"));
	}

	let file_length = data.seek(SeekFrom::Current(0))?;

	let first_mpeg_frame = (first_mpeg_frame.0.unwrap(), first_mpeg_frame.1);
	let last_mpeg_frame = (last_mpeg_frame.0.unwrap(), last_mpeg_frame.1);

	let xing_header_location = first_mpeg_frame.1 + u64::from(first_mpeg_frame.0.data_start);

	data.seek(SeekFrom::Start(xing_header_location))?;

	let mut xing_reader = [0; 32];
	data.read_exact(&mut xing_reader)?;

	let xing_header = XingHeader::read(&mut &xing_reader[..]).ok();

	Ok(Mp3File {
		#[cfg(feature = "id3v2")]
		id3v2_tag,
		#[cfg(feature = "id3v1")]
		id3v1_tag,
		#[cfg(feature = "ape")]
		ape_tag,
		properties: read_properties(first_mpeg_frame, last_mpeg_frame, xing_header, file_length),
	})
}
