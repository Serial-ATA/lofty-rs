use super::read::verify_aiff;
use crate::error::{LoftyError, Result};
use crate::types::item::ItemKey;
use crate::types::tag::{ItemValue, Tag, TagType};

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};

use byteorder::{BigEndian, LittleEndian, ReadBytesExt};

// TODO: support ID3v2
pub(in crate::logic) fn write_to(data: &mut File, tag: &Tag) -> Result<()> {
	if tag.tag_type() != &TagType::AiffText {
		return Err(LoftyError::UnsupportedTag);
	}

	verify_aiff(data)?;

	let mut text_chunks = Vec::new();

	let items = tag.items().iter().filter(|i| {
		(i.key() == &ItemKey::TrackTitle
			|| i.key() == &ItemKey::TrackArtist
			|| i.key() == &ItemKey::CopyrightMessage)
			&& std::mem::discriminant(i.value())
				== std::mem::discriminant(&ItemValue::Text(String::new()))
	});

	for i in items {
		// Already covered
		let value = match i.value() {
			ItemValue::Text(value) => value,
			_ => unreachable!(),
		};

		let len = (value.len() as u32).to_be_bytes();

		// Safe to unwrap since we retained the only possible values
		text_chunks.extend(
			i.key()
				.map_key(&TagType::AiffText)
				.unwrap()
				.as_bytes()
				.iter(),
		);
		text_chunks.extend(len.iter());
		text_chunks.extend(value.as_bytes().iter());
	}

	let mut chunks_remove = Vec::new();

	while let (Ok(fourcc), Ok(size)) = (
		data.read_u32::<LittleEndian>(),
		data.read_u32::<BigEndian>(),
	) {
		let fourcc_b = &fourcc.to_le_bytes();
		let pos = (data.seek(SeekFrom::Current(0))? - 8) as usize;

		if fourcc_b == b"NAME" || fourcc_b == b"AUTH" || fourcc_b == b"(c) " {
			chunks_remove.push((pos, (pos + 8 + size as usize)))
		}

		data.seek(SeekFrom::Current(i64::from(size)))?;
	}

	data.seek(SeekFrom::Start(0))?;

	let mut file_bytes = Vec::new();
	data.read_to_end(&mut file_bytes)?;

	if chunks_remove.is_empty() {
		data.seek(SeekFrom::Start(16))?;

		let mut size = [0; 4];
		data.read_exact(&mut size)?;

		let comm_end = (20 + u32::from_le_bytes(size)) as usize;
		file_bytes.splice(comm_end..comm_end, text_chunks);
	} else {
		chunks_remove.sort_unstable();
		chunks_remove.reverse();

		let first = chunks_remove.pop().unwrap();

		for (s, e) in &chunks_remove {
			file_bytes.drain(*s as usize..*e as usize);
		}

		file_bytes.splice(first.0 as usize..first.1 as usize, text_chunks);
	}

	let total_size = ((file_bytes.len() - 8) as u32).to_be_bytes();
	file_bytes.splice(4..8, total_size.to_vec());

	data.seek(SeekFrom::Start(0))?;
	data.set_len(0)?;
	data.write_all(&*file_bytes)?;

	Ok(())
}
