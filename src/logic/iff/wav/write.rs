use super::read::verify_riff;
use crate::error::{LoftyError, Result};
use crate::types::tag::Tag;

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};

use byteorder::{LittleEndian, ReadBytesExt};

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

// TODO
// pub(crate) fn write_to(data: &mut File, metadata: &Tag) -> Result<()> {
// 	let mut packet = Vec::new();
//
// 	packet.extend(b"LIST".iter());
// 	packet.extend(b"INFO".iter());
//
// 	for (k, v) in metadata {
// 		let mut val = v.as_bytes().to_vec();
//
// 		if val.len() % 2 != 0 {
// 			val.push(0)
// 		}
//
// 		let size = val.len() as u32;
//
// 		packet.extend(k.as_bytes().iter());
// 		packet.extend(size.to_le_bytes().iter());
// 		packet.extend(val.iter());
// 	}
//
// 	let packet_size = packet.len() - 4;
//
// 	if packet_size > u32::MAX as usize {
// 		return Err(LoftyError::TooMuchData);
// 	}
//
// 	let size = (packet_size as u32).to_le_bytes();
//
// 	#[allow(clippy::needless_range_loop)]
// 	for i in 0..4 {
// 		packet.insert(i + 4, size[i]);
// 	}
//
// 	verify_riff(data)?;
//
// 	data.seek(SeekFrom::Current(8))?;
//
// 	find_info_list(data)?;
//
// 	let info_list_size = data.read_u32::<LittleEndian>()? as usize;
// 	data.seek(SeekFrom::Current(-8))?;
//
// 	let info_list_start = data.seek(SeekFrom::Current(0))? as usize;
// 	let info_list_end = info_list_start + 8 + info_list_size;
//
// 	data.seek(SeekFrom::Start(0))?;
// 	let mut file_bytes = Vec::new();
// 	data.read_to_end(&mut file_bytes)?;
//
// 	let _ = file_bytes.splice(info_list_start..info_list_end, packet);
//
// 	let total_size = (file_bytes.len() - 8) as u32;
// 	let _ = file_bytes.splice(4..8, total_size.to_le_bytes().to_vec());
//
// 	data.seek(SeekFrom::Start(0))?;
// 	data.set_len(0)?;
// 	data.write_all(&*file_bytes)?;
//
// 	Ok(())
// }
