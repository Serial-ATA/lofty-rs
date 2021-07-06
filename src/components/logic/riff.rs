use crate::{LoftyError, Result};

use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

// Used to determine the RIFF metadata format
pub const LIST_ID: &[u8] = b"LIST";

// FourCC

// Standard
pub const IART: &[u8] = &[73, 65, 82, 84];
pub const ICMT: &[u8] = &[73, 67, 77, 84];
pub const ICRD: &[u8] = &[73, 67, 82, 68];
pub const INAM: &[u8] = &[73, 78, 65, 77];
pub const IPRD: &[u8] = &[73, 80, 82, 68]; // Represents album title

// Non-standard
pub const ITRK: &[u8] = &[73, 84, 82, 75]; // Can represent track number
pub const IPRT: &[u8] = &[73, 80, 82, 84]; // Can also represent track number
pub const IFRM: &[u8] = &[73, 70, 82, 77]; // Can represent total tracks

// Very non-standard
pub const ALBU: &[u8] = &[65, 76, 66, 85]; // Can album artist OR album title
pub const TRAC: &[u8] = &[84, 82, 65, 67]; // Can represent track number OR total tracks
pub const DISC: &[u8] = &[68, 73, 83, 67]; // Can represent disc number OR total discs

pub const NULL_CHAR: char = '\0';

pub(crate) fn read_from<T>(data: &mut T) -> Result<Option<HashMap<String, String>>>
where
	T: Read + Seek,
{
	verify_riff(data)?;

	data.seek(SeekFrom::Current(8))?;

	find_info_list(data)?;

	let info_list_size = data.read_u32::<LittleEndian>()?;

	let mut info_list = vec![0; info_list_size as usize];
	data.read_exact(&mut info_list)?;

	let mut cursor = Cursor::new(&*info_list);
	cursor.seek(SeekFrom::Start(4))?; // Skip the chunk ID

	let mut metadata: HashMap<String, String> = HashMap::new();

	let mut reading = true;

	#[allow(clippy::cast_lossless)]
	while reading {
		if let (Ok(fourcc), Ok(size)) = (
			cursor.read_u32::<LittleEndian>(),
			cursor.read_u32::<LittleEndian>(),
		) {
			match create_key(&fourcc.to_le_bytes()) {
				Some(key) => {
					let mut buf = vec![0; size as usize];
					cursor.read_exact(&mut buf)?;

					match std::string::String::from_utf8(buf) {
						Ok(val) => {
							let _ = metadata.insert(key, val.trim_matches(NULL_CHAR).to_string());
						},
						Err(_) => {
							return Err(LoftyError::InvalidData(
								"RIFF file contains non UTF-8 strings",
							))
						},
					}
				},
				None => cursor.set_position(cursor.position() + size as u64),
			}

			if cursor.position() >= info_list_size as u64 {
				reading = false
			}
		} else {
			reading = false
		}
	}

	Ok(Some(metadata))
}

fn find_info_list<T>(data: &mut T) -> Result<()>
where
	T: Read + Seek,
{
	loop {
		let mut chunk_name = [0; 4];
		data.read_exact(&mut chunk_name)?;

		if chunk_name == LIST_ID {
			data.seek(SeekFrom::Current(4))?;

			let mut list_type = [0; 4];
			data.read_exact(&mut list_type)?;

			if &list_type == b"INFO" {
				data.seek(SeekFrom::Current(-8))?;
				return Ok(());
			}

			data.seek(SeekFrom::Current(-8))?;
		}

		let size = data.read_u32::<LittleEndian>()?;
		data.seek(SeekFrom::Current(i64::from(size)))?;
	}
}

fn verify_riff<T>(data: &mut T) -> Result<()>
where
	T: Read + Seek,
{
	let mut id = [0; 4];
	data.read_exact(&mut id)?;

	if &id != b"RIFF" {
		return Err(LoftyError::Riff("RIFF file doesn't contain a RIFF chunk"));
	}

	Ok(())
}

fn create_key(fourcc: &[u8]) -> Option<String> {
	match fourcc {
		IART => Some("Artist".to_string()),
		ICMT => Some("Comment".to_string()),
		ICRD => Some("Date".to_string()),
		INAM => Some("Title".to_string()),
		IPRD | ALBU => Some("Album".to_string()),
		ITRK | IPRT | TRAC => Some("TrackNumber".to_string()),
		IFRM => Some("TrackTotal".to_string()),
		DISC => Some("DiscNumber".to_string()),
		_ => None,
	}
}

pub fn key_to_fourcc(key: &str) -> Option<&[u8]> {
	match key {
		"Artist" => Some(IART),
		"Comment" => Some(ICMT),
		"Date" => Some(ICRD),
		"Title" => Some(INAM),
		"Album" => Some(IPRD),
		"TrackTotal" => Some(IFRM),
		"TrackNumber" => Some(TRAC),
		"DiscNumber" | "DiscTotal" => Some(DISC),
		_ => None,
	}
}

#[cfg(feature = "format-riff")]
pub(crate) fn write_to(data: &mut File, metadata: HashMap<String, String>) -> Result<()> {
	let mut packet = Vec::new();

	packet.extend(LIST_ID.iter());
	packet.extend(b"INFO".iter());

	for (k, v) in metadata {
		if let Some(fcc) = key_to_fourcc(&*k) {
			let mut val = v.as_bytes().to_vec();

			if val.len() % 2 != 0 {
				val.push(0)
			}

			let size = val.len() as u32;

			packet.extend(fcc.iter());
			packet.extend(size.to_le_bytes().iter());
			packet.extend(val.iter());
		}
	}

	let packet_size = packet.len() - 4;

	if packet_size > u32::MAX as usize {
		return Err(LoftyError::TooMuchData);
	}

	let size = (packet_size as u32).to_le_bytes();

	#[allow(clippy::needless_range_loop)]
	for i in 0..4 {
		packet.insert(i + 4, size[i]);
	}

	verify_riff(data)?;

	data.seek(SeekFrom::Current(8))?;

	find_info_list(data)?;

	let info_list_size = data.read_u32::<LittleEndian>()? as usize;
	data.seek(SeekFrom::Current(-8))?;

	let info_list_start = data.seek(SeekFrom::Current(0))? as usize;
	let info_list_end = info_list_start + 8 + info_list_size;

	data.seek(SeekFrom::Start(0))?;
	let mut file_bytes = Vec::new();
	data.read_to_end(&mut file_bytes)?;

	let _ = file_bytes.splice(info_list_start..info_list_end, packet);

	let total_size = (file_bytes.len() - 8) as u32;
	let _ = file_bytes.splice(4..8, total_size.to_le_bytes().to_vec());

	data.seek(SeekFrom::Start(0))?;
	data.set_len(0)?;
	data.write_all(&*file_bytes)?;

	Ok(())
}
