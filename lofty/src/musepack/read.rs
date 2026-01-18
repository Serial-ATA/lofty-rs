use super::sv4to6::MpcSv4to6Properties;
use super::sv7::MpcSv7Properties;
use super::sv8::MpcSv8Properties;
use super::{MpcFile, MpcProperties, MpcStreamVersion};
use crate::config::ParseOptions;
use crate::error::Result;
use crate::id3::v2::read::parse_id3v2;
use crate::id3::{FindId3v2Config, ID3FindResults, find_id3v1, find_id3v2, find_lyrics3v2};
use crate::macros::err;
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

	let find_id3v2_config = if parse_options.read_tags {
		FindId3v2Config::READ_TAG
	} else {
		FindId3v2Config::NO_READ_TAG
	};

	// ID3v2 tags are unsupported in MPC files, but still possible
	#[allow(unused_variables)]
	if let ID3FindResults(Some(header), Some(content)) = find_id3v2(reader, find_id3v2_config)? {
		let Some(new_stream_length) = stream_length.checked_sub(u64::from(header.full_tag_size()))
		else {
			err!(SizeMismatch);
		};

		stream_length = new_stream_length;

		let reader = &mut &*content;

		let id3v2 = parse_id3v2(reader, header, parse_options)?;
		file.id3v2_tag = Some(id3v2);
	}

	// Save the current position, so we can go back and read the properties after the tags
	let pos_past_id3v2 = reader.stream_position()?;

	#[allow(unused_variables)]
	let ID3FindResults(header, id3v1) =
		find_id3v1(reader, parse_options.read_tags, parse_options.parsing_mode)?;

	if header.is_some() {
		file.id3v1_tag = id3v1;
		let Some(new_stream_length) = stream_length.checked_sub(128) else {
			err!(SizeMismatch);
		};

		stream_length = new_stream_length;
	}

	let ID3FindResults(_, lyrics3v2_size) = find_lyrics3v2(reader)?;
	let Some(new_stream_length) = stream_length.checked_sub(u64::from(lyrics3v2_size)) else {
		err!(SizeMismatch);
	};

	stream_length = new_stream_length;

	reader.seek(SeekFrom::Current(-32))?;

	if let (tag, Some(header)) = crate::ape::tag::read::read_ape_tag(reader, true, parse_options)? {
		file.ape_tag = tag;

		// Seek back to the start of the tag
		let pos = reader.stream_position()?;

		let tag_size = u64::from(header.size);
		let Some(tag_start) = pos.checked_sub(tag_size) else {
			err!(SizeMismatch);
		};

		reader.seek(SeekFrom::Start(tag_start))?;

		let Some(new_stream_length) = stream_length.checked_sub(tag_size) else {
			err!(SizeMismatch);
		};
		stream_length = new_stream_length;
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
