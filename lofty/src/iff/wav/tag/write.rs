use super::RIFFInfoListRef;
use crate::config::WriteOptions;
use crate::error::{LoftyError, Result};
use crate::iff::chunk::Chunks;
use crate::iff::wav::read::verify_wav;
use crate::macros::err;
use crate::util::io::{FileLike, Length, Truncate};

use std::io::{Read, Seek, SeekFrom};

use byteorder::{LittleEndian, WriteBytesExt};

pub(in crate::iff::wav) fn write_riff_info<'a, F, I>(
	file: &mut F,
	tag: &mut RIFFInfoListRef<'a, I>,
	_write_options: WriteOptions,
) -> Result<()>
where
	F: FileLike,
	LoftyError: From<<F as Truncate>::Error>,
	LoftyError: From<<F as Length>::Error>,
	I: Iterator<Item = (&'a str, &'a str)>,
{
	verify_wav(file)?;
	let file_len = file.len()?.saturating_sub(12);

	let mut riff_info_bytes = Vec::new();
	create_riff_info(&mut tag.items, &mut riff_info_bytes)?;

	let Some(info_list_size) = find_info_list(file, file_len)? else {
		// Simply append the info list to the end of the file and update the file size
		file.seek(SeekFrom::End(0))?;

		file.write_all(&riff_info_bytes)?;

		let len = (file.stream_position()? - 8) as u32;

		file.seek(SeekFrom::Start(4))?;
		file.write_u32::<LittleEndian>(len)?;

		return Ok(());
	};

	// Replace the existing tag

	let info_list_start = file.seek(SeekFrom::Current(-12))? as usize;
	let info_list_end = info_list_start + 8 + info_list_size as usize;

	file.rewind()?;

	let mut file_bytes = Vec::new();
	file.read_to_end(&mut file_bytes)?;

	let _ = file_bytes.splice(info_list_start..info_list_end, riff_info_bytes);

	let total_size = (file_bytes.len() - 8) as u32;
	let _ = file_bytes.splice(4..8, total_size.to_le_bytes());

	file.rewind()?;
	file.truncate(0)?;
	file.write_all(&file_bytes)?;

	Ok(())
}

fn find_info_list<R>(data: &mut R, file_size: u64) -> Result<Option<u32>>
where
	R: Read + Seek,
{
	let mut info = None;

	let mut chunks = Chunks::<LittleEndian>::new(file_size);

	while chunks.next(data).is_ok() {
		if &chunks.fourcc == b"LIST" {
			let mut list_type = [0; 4];
			data.read_exact(&mut list_type)?;

			if &list_type == b"INFO" {
				log::debug!("Found existing RIFF INFO list, size: {} bytes", chunks.size);

				info = Some(chunks.size);
				break;
			}

			data.seek(SeekFrom::Current(-8))?;
		}

		chunks.skip(data)?;
	}

	Ok(info)
}

pub(super) fn create_riff_info(
	items: &mut dyn Iterator<Item = (&str, &str)>,
	bytes: &mut Vec<u8>,
) -> Result<()> {
	let mut items = items.peekable();

	if items.peek().is_none() {
		log::debug!("No items to write, removing RIFF INFO list");
		return Ok(());
	}

	bytes.extend(b"LIST");
	bytes.extend(b"INFO");

	for (k, v) in items {
		if v.is_empty() {
			continue;
		}

		let val_b = v.as_bytes();
		// Account for null terminator
		let len = val_b.len() + 1;

		// Each value has to be null terminated and have an even length
		let terminator: &[u8] = if len % 2 == 0 { &[0] } else { &[0, 0] };

		bytes.extend(k.as_bytes());
		bytes.extend(&(len as u32).to_le_bytes());
		bytes.extend(val_b);
		bytes.extend(terminator);
	}

	let packet_size = Vec::len(bytes) - 4;

	if packet_size > u32::MAX as usize {
		err!(TooMuchData);
	}

	log::debug!("Created RIFF INFO list, size: {} bytes", packet_size);
	let size = (packet_size as u32).to_le_bytes();

	#[allow(clippy::needless_range_loop)]
	for i in 0..4 {
		bytes.insert(i + 4, size[i]);
	}

	Ok(())
}
