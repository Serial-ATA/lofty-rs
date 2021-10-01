use crate::error::Result;

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};

use byteorder::{ByteOrder, ReadBytesExt, WriteBytesExt};

pub(in crate::logic::id3::v2) fn write_to_chunk_file<B>(data: &mut File, tag: &[u8]) -> Result<()>
where
	B: ByteOrder,
{
	let mut id3v2_chunk = (None, None);

	let mut fourcc = [0; 4];

	while let (Ok(()), Ok(size)) = (data.read_exact(&mut fourcc), data.read_u32::<B>()) {
		if &fourcc == b"ID3 " || &fourcc == b"id3 " {
			id3v2_chunk = (Some(data.seek(SeekFrom::Current(0))? - 8), Some(size));
			break;
		}

		data.seek(SeekFrom::Current(i64::from(size)))?;
	}

	if let (Some(chunk_start), Some(chunk_size)) = id3v2_chunk {
		data.seek(SeekFrom::Start(0))?;

		let mut file_bytes = Vec::new();
		data.read_to_end(&mut file_bytes)?;

		file_bytes.splice(
			chunk_start as usize..(chunk_start + u64::from(chunk_size) + 8) as usize,
			[],
		);

		data.seek(SeekFrom::Start(0))?;
		data.set_len(0)?;
		data.write_all(&*file_bytes)?;
	}

	if !tag.is_empty() {
		data.seek(SeekFrom::End(0))?;
		data.write_all(&[b'I', b'D', b'3', b' '])?;
		data.write_u32::<B>(tag.len() as u32)?;
		data.write_all(tag)?;

		let total_size = data.seek(SeekFrom::Current(0))? - 8;
		data.seek(SeekFrom::Start(4))?;

		data.write_u32::<B>(total_size as u32)?;
	}

	Ok(())
}
