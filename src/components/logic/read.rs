use super::constants::{ID3_ID, LIST_ID};
use crate::{Error, Result, ToAnyTag};
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

		for _ in 0..chunk_len {
			let fourcc = cursor.read_u32::<LittleEndian>()? as u32;
			let size = cursor.read_u32::<LittleEndian>()? as u32;

			match create_wav_key(&fourcc.to_le_bytes()) {
				Some(key) => {
					let mut buf = vec![0; size as usize];
					cursor.read_exact(&mut buf)?;

					let val = std::str::from_utf8(&*buf)?;
					metadata.insert(key, val.trim_matches(char::from(0)).to_string());

					// Skip null byte
					if size as usize % 2 != 0 {
						cursor.set_position(cursor.position() + 1)
					}
				},
				None => cursor.set_position(cursor.position() + u64::from(size)),
			}
		}

		Ok(Some(metadata))
	} else {
		Err(Error::Wav(
			"This file does not contain an INFO chunk".to_string(),
		))
	};
}

fn create_wav_key(fourcc: &[u8]) -> Option<String> {
	match fourcc {
		fcc if fcc == super::constants::IART => Some("Artist".to_string()),
		fcc if fcc == super::constants::ICMT => Some("Comment".to_string()),
		fcc if fcc == super::constants::ICRD => Some("Date".to_string()),
		fcc if fcc == super::constants::INAM => Some("Title".to_string()),
		fcc if fcc == super::constants::ISFT => Some("Title".to_string()),
		_ => None,
	}
}
