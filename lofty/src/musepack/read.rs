use super::sv4to6::MpcSv4to6Properties;
use super::sv7::MpcSv7Properties;
use super::sv8::MpcSv8Properties;
use super::{MpcFile, MpcProperties, MpcStreamVersion};
use crate::config::ParseOptions;
use crate::error::Result;
use crate::id3::v2::read::parse_id3v2;
use crate::id3::{find_id3v1, find_id3v2, find_lyrics3v2, FindId3v2Config, ID3FindResults};
use crate::util::io::SeekStreamLen;

use std::io::{Read, Seek, SeekFrom};

pub(super) fn read_from<R>(reader: &mut R, parse_options: ParseOptions) -> Result<MpcFile>
where
	R: Read + Seek,
{
	log::debug!("Attempting to read MPC file");

	// Sv4to6 is the default, as it doesn't have a marker like Sv8's b'MPCK' or Sv7's b'MP+'
	let mut version = MpcStreamVersion::Sv4to6;
	let mut file = MpcFile::default();

	let mut stream_length = reader.stream_len_hack()?;

	// ID3v2 tags are unsupported in MPC files, but still possible
	#[allow(unused_variables)]
	if let ID3FindResults(Some(header), Some(content)) =
		find_id3v2(reader, FindId3v2Config::READ_TAG)?
	{
		let reader = &mut &*content;

		let id3v2 = parse_id3v2(reader, header, parse_options.parsing_mode)?;
		file.id3v2_tag = Some(id3v2);

		let mut size = header.size;
		if header.flags.footer {
			size += 10;
		}

		stream_length -= u64::from(size);
	}

	// Save the current position, so we can go back and read the properties after the tags
	let pos_past_id3v2 = reader.stream_position()?;

	#[allow(unused_variables)]
	let ID3FindResults(header, id3v1) = find_id3v1(reader, true)?;

	if header.is_some() {
		file.id3v1_tag = id3v1;
		stream_length -= 128;
	}

	let ID3FindResults(_, lyrics3v2_size) = find_lyrics3v2(reader)?;
	stream_length -= u64::from(lyrics3v2_size);

	reader.seek(SeekFrom::Current(-32))?;

	if let Some((tag, header)) = crate::ape::tag::read::read_ape_tag(reader, true)? {
		file.ape_tag = Some(tag);

		// Seek back to the start of the tag
		let pos = reader.stream_position()?;
		reader.seek(SeekFrom::Start(pos - u64::from(header.size)))?;

		stream_length -= u64::from(header.size);
	}

	// Restore the position of the magic signature
	reader.seek(SeekFrom::Start(pos_past_id3v2))?;

	let mut header = [0; 4];
	reader.read_exact(&mut header)?;

	match &header {
		b"MPCK" => {
			log::debug!("MPC stream version determined to be 8");
			version = MpcStreamVersion::Sv8;
		},
		[b'M', b'P', b'+', ..] => {
			log::debug!("MPC stream version determined to be 7");

			// Seek back the extra byte we read
			reader.seek(SeekFrom::Current(-1))?;
			version = MpcStreamVersion::Sv7;
		},
		_ => {
			log::warn!("MPC stream version could not be determined, assuming 4-6");

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
			MpcStreamVersion::Sv7 => {
				file.properties = MpcProperties::Sv7(MpcSv7Properties::read(reader, stream_length)?)
			},
			MpcStreamVersion::Sv4to6 => {
				file.properties = MpcProperties::Sv4to6(MpcSv4to6Properties::read(
					reader,
					parse_options.parsing_mode,
					stream_length,
				)?)
			},
		}
	}

	Ok(file)
}
