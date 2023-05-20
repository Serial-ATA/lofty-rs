use super::sv8::MpcSv8Properties;
use super::{MpcFile, MpcProperties, MpcStreamVersion};
use crate::error::Result;
use crate::id3::v2::read::parse_id3v2;
use crate::id3::{find_id3v1, find_id3v2, find_lyrics3v2, ID3FindResults};
use crate::probe::ParseOptions;

use std::io::{Read, Seek, SeekFrom};

pub(super) fn read_from<R>(reader: &mut R, parse_options: ParseOptions) -> Result<MpcFile>
where
	R: Read + Seek,
{
	// Sv4to6 is the default, as it doesn't have a marker like Sv8's b'MPCK' or Sv7's b'MP+'
	let mut version = MpcStreamVersion::Sv4to6;
	let mut file = MpcFile::default();

	// ID3v2 tags are unsupported in MPC files, but still possible
	#[allow(unused_variables)]
	if let ID3FindResults(Some(header), Some(content)) = find_id3v2(reader, true)? {
		let reader = &mut &*content;

		let id3v2 = parse_id3v2(reader, header)?;
		file.id3v2_tag = Some(id3v2);
	}

	let mut header = [0; 4];
	reader.read_exact(&mut header)?;

	match &header {
		b"MPCK" => {
			version = MpcStreamVersion::Sv8;
		},
		[b'M', b'P', b'+', ..] => {
			// Seek back the extra byte we read
			reader.seek(SeekFrom::Current(-1))?;
			version = MpcStreamVersion::Sv7;
		},
		_ => {
			// We should be reading into the actual content now, seek back
			reader.seek(SeekFrom::Current(-4))?;
		},
	}

	if parse_options.read_properties {
		match version {
			MpcStreamVersion::Sv8 => {
				file.properties =
					MpcProperties::Sv8(MpcSv8Properties::read(reader, parse_options.parsing_mode)?)
			},
			MpcStreamVersion::Sv7 => todo!(),
			MpcStreamVersion::Sv4to6 => todo!(),
		}
	}

	#[allow(unused_variables)]
	let ID3FindResults(header, id3v1) = find_id3v1(reader, true)?;

	if header.is_some() {
		file.id3v1_tag = id3v1;
	}

	let _ = find_lyrics3v2(reader)?;

	reader.seek(SeekFrom::Current(-32))?;

	match crate::ape::tag::read::read_ape_tag(reader, true)? {
		Some((tag, header)) => {
			file.ape_tag = Some(tag);

			// Seek back to the start of the tag
			let pos = reader.stream_position()?;
			reader.seek(SeekFrom::Start(pos - u64::from(header.size)))?;
		},
		None => {
			// Correct the position (APE header - Preamble)
			reader.seek(SeekFrom::Current(24))?;
		},
	}

	Ok(file)
}
