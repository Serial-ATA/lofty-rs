use crate::{LoftyError, Result};

use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use byteorder::{LittleEndian, ReadBytesExt};

pub(crate) fn read_from<T>(data: &mut T) -> Result<HashMap<String, String>>
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

	#[allow(clippy::cast_lossless)]
	while cursor.position() < info_list_size as u64 {
		let mut fourcc = vec![0; 4];
		cursor.read_exact(&mut fourcc)?;

		let size = cursor.read_u32::<LittleEndian>()?;

		let key = String::from_utf8(fourcc)?;

		let mut buf = vec![0; size as usize];
		cursor.read_exact(&mut buf)?;

		let val = String::from_utf8(buf)?;
		metadata.insert(key.to_string(), val.trim_matches('\0').to_string());
	}

	Ok(metadata)
}

fn find_info_list<T>(data: &mut T) -> Result<()>
where
	T: Read + Seek,
{
	loop {
		let mut chunk_name = [0; 4];
		data.read_exact(&mut chunk_name)?;

		if &chunk_name == b"LIST" {
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

#[cfg(feature = "format-riff")]
pub(crate) fn write_to(data: &mut File, metadata: HashMap<String, String>) -> Result<()> {
	let mut packet = Vec::new();

	packet.extend(b"LIST".iter());
	packet.extend(b"INFO".iter());

	for (k, v) in metadata {
		let mut val = v.as_bytes().to_vec();

		if val.len() % 2 != 0 {
			val.push(0)
		}

		let size = val.len() as u32;

		packet.extend(k.as_bytes().iter());
		packet.extend(size.to_le_bytes().iter());
		packet.extend(val.iter());
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
