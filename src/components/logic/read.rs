use super::constants::{ID3_ID, LIST_ID};
use crate::{Error, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::HashMap;
use std::io::{Cursor, Read, Seek};

pub(crate) fn wav<T>(mut data: T) -> Result<Option<HashMap<String, String>>>
where
	T: Read + Seek,
{
	let chunk = riff::Chunk::read(&mut data, 0)?;

	let mut list: Option<riff::Chunk> = None;

	for child in chunk.iter(&mut data) {
		let chunk_id = child.id();
		let value_upper = std::str::from_utf8(&chunk_id.value)?.to_uppercase();
		let value_bytes = value_upper.as_bytes();

		if value_bytes == LIST_ID {
			// TODO: actually check for the INFO id rather than any LIST
			list = Some(child);
			break;
		}

		if value_bytes == ID3_ID {
			#[cfg(feature = "mp3")]
			{
				list = Some(child);
				break;
			}

			#[cfg(not(feature = "mp3"))]
			return Err(Error::Wav(
				"WAV file has an id3 tag, but `mp3` feature is not enabled.",
			));
		}
	}

	return if let Some(list) = list {
		let mut content = list.read_contents(&mut data)?;

		#[cfg(feature = "mp3")]
		if &list.id().value == ID3_ID {
			// TODO
		}

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
				match create_wav_key(&fourcc.to_le_bytes()) {
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
		Err(Error::Wav(
			"This file doesn't contain an INFO chunk".to_string(),
		))
	};
}

fn create_wav_key(fourcc: &[u8]) -> Option<String> {
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
