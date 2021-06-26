use super::constants::LIST_ID;
use crate::{LoftyError, Result};

use byteorder::{LittleEndian, ReadBytesExt};
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

pub(crate) fn read_from<T>(mut data: T) -> Result<Option<HashMap<String, String>>>
where
	T: Read + Seek,
{
	let chunk = riff::Chunk::read(&mut data, 0)?;

	let mut lists: Vec<riff::Chunk> = Vec::new();

	for child in chunk.iter(&mut data) {
		let chunk_id = child.id();

		if &chunk_id.value == LIST_ID {
			lists.push(child)
		}
	}

	return if lists.is_empty() {
		Err(LoftyError::Riff("This file doesn't contain a LIST chunk"))
	} else {
		let mut info: Option<riff::Chunk> = None;

		for child in lists {
			if &child.read_type(&mut data)?.value == b"INFO" {
				info = Some(child);
				break;
			}
		}

		if let Some(list) = info {
			let mut content = list.read_contents(&mut data)?;

			content.drain(0..4); // Get rid of the chunk ID
			let mut cursor = Cursor::new(&*content);

			let chunk_len = list.len();
			let mut metadata: HashMap<String, String> = HashMap::with_capacity(chunk_len as usize);

			let mut reading = true;

			while reading {
				if let (Ok(fourcc), Ok(size)) = (
					cursor.read_u32::<LittleEndian>(),
					cursor.read_u32::<LittleEndian>(),
				) {
					match create_key(&fourcc.to_le_bytes()) {
						Some(key) => {
							let mut buf = vec![0; size as usize];
							cursor.read_exact(&mut buf)?;

							// Just skip any values that can't be converted
							match std::string::String::from_utf8(buf) {
								Ok(val) => {
									let _ = metadata
										.insert(key, val.trim_matches(char::from(0)).to_string());
								},
								Err(_) => continue,
							}
						},
						#[allow(clippy::cast_lossless)]
						None => cursor.set_position(cursor.position() + size as u64),
					}

					// Skip null byte
					if size as usize % 2 != 0 {
						cursor.set_position(cursor.position() + 1)
					}

					if cursor.position() >= cursor.get_ref().len() as u64 {
						reading = false
					}
				} else {
					reading = false
				}
			}

			Ok(Some(metadata))
		} else {
			Err(LoftyError::Riff("This file doesn't contain an INFO chunk"))
		}
	};
}

fn create_key(fourcc: &[u8]) -> Option<String> {
	match fourcc {
		fcc if fcc == super::constants::IART => Some("Artist".to_string()),
		fcc if fcc == super::constants::ICMT => Some("Comment".to_string()),
		fcc if fcc == super::constants::ICRD => Some("Date".to_string()),
		fcc if fcc == super::constants::INAM => Some("Title".to_string()),
		fcc if fcc == super::constants::IPRD => Some("Album".to_string()),

		// Non-standard
		fcc if fcc == super::constants::ITRK || fcc == super::constants::IPRT => {
			Some("TrackNumber".to_string())
		},
		fcc if fcc == super::constants::IFRM => Some("TrackTotal".to_string()),
		fcc if fcc == super::constants::ALBU => Some("Album".to_string()),
		fcc if fcc == super::constants::TRAC => Some("TrackNumber".to_string()),
		fcc if fcc == super::constants::DISC => Some("DiscNumber".to_string()),
		_ => None,
	}
}

pub fn key_to_fourcc(key: &str) -> Option<[u8; 4]> {
	match key {
		"Artist" => Some(super::constants::IART),
		"Comment" => Some(super::constants::ICMT),
		"Date" => Some(super::constants::ICRD),
		"Title" => Some(super::constants::INAM),
		"Album" => Some(super::constants::IPRD),
		"TrackTotal" => Some(super::constants::IFRM),
		"TrackNumber" => Some(super::constants::TRAC),
		"DiscNumber" | "DiscTotal" => Some(super::constants::DISC),
		_ => None,
	}
}

#[cfg(feature = "format-riff")]
pub(crate) fn write_to(data: &mut File, metadata: HashMap<String, String>) -> Result<()> {
	let mut packet = Vec::new();

	packet.extend(riff::LIST_ID.value.iter());

	let fourcc = "INFO";
	packet.extend(fourcc.as_bytes().iter());

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

	let mut file_bytes = Vec::new();
	std::io::copy(data.borrow_mut(), &mut file_bytes)?;

	let len = (packet.len() - 4) as u32;
	let size = len.to_le_bytes();

	#[allow(clippy::needless_range_loop)]
	for i in 0..4 {
		packet.insert(i + 4, size[i]);
	}

	let mut file = Cursor::new(file_bytes);

	let chunk = riff::Chunk::read(&mut file, 0)?;

	let (mut list_pos, mut list_len): (Option<u32>, Option<u32>) = (None, None);

	if chunk.id() != riff::RIFF_ID {
		return Err(LoftyError::Riff("This file does not contain a RIFF chunk"));
	}

	for child in chunk.iter(&mut file) {
		if child.id() == riff::LIST_ID {
			list_pos = Some(child.offset() as u32);
			list_len = Some(child.len());
		}
	}

	file.seek(SeekFrom::Start(0))?;

	let mut content = Vec::new();
	std::io::copy(&mut file, &mut content)?;

	if let (Some(list_pos), Some(list_len)) = (list_pos, list_len) {
		let list_end = (list_pos + list_len) as usize;

		let _ = content.splice(list_pos as usize..list_end, packet);

		let total_size = (content.len() - 8) as u32;
		let _ = content.splice(4..8, total_size.to_le_bytes().to_vec());

		data.seek(SeekFrom::Start(0))?;
		data.set_len(0)?;
		data.write_all(&*content)?;

		Ok(())
	} else {
		Err(LoftyError::Riff("This file does not contain an INFO chunk"))
	}
}
