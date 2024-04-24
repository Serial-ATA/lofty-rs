use crate::config::WriteOptions;
use crate::error::{LoftyError, Result};
use crate::iff::chunk::Chunks;
use crate::util::io::{FileLike, Length, Truncate};

use std::io::SeekFrom;

use byteorder::{ByteOrder, WriteBytesExt};

const CHUNK_NAME_UPPER: [u8; 4] = [b'I', b'D', b'3', b' '];
const CHUNK_NAME_LOWER: [u8; 4] = [b'i', b'd', b'3', b' '];

pub(in crate::id3::v2) fn write_to_chunk_file<F, B>(
	file: &mut F,
	tag: &[u8],
	write_options: WriteOptions,
) -> Result<()>
where
	F: FileLike,
	LoftyError: From<<F as Truncate>::Error>,
	LoftyError: From<<F as Length>::Error>,
	B: ByteOrder,
{
	// RIFF....WAVE
	file.seek(SeekFrom::Current(12))?;

	let file_len = file.len()?.saturating_sub(12);

	let mut id3v2_chunk = (None, None);

	let mut chunks = Chunks::<B>::new(file_len);

	while chunks.next(file).is_ok() {
		if chunks.fourcc == CHUNK_NAME_UPPER || chunks.fourcc == CHUNK_NAME_LOWER {
			id3v2_chunk = (Some(file.stream_position()? - 8), Some(chunks.size));
			break;
		}

		file.seek(SeekFrom::Current(i64::from(chunks.size)))?;

		chunks.correct_position(file)?;
	}

	if let (Some(chunk_start), Some(mut chunk_size)) = id3v2_chunk {
		file.rewind()?;

		// We need to remove the padding byte if it exists
		if chunk_size % 2 != 0 {
			chunk_size += 1;
		}

		let mut file_bytes = Vec::new();
		file.read_to_end(&mut file_bytes)?;

		file_bytes.splice(
			chunk_start as usize..(chunk_start + u64::from(chunk_size) + 8) as usize,
			[],
		);

		file.rewind()?;
		file.truncate(0)?;
		file.write_all(&file_bytes)?;
	}

	if !tag.is_empty() {
		file.seek(SeekFrom::End(0))?;

		if write_options.uppercase_id3v2_chunk {
			file.write_all(&CHUNK_NAME_UPPER)?;
		} else {
			file.write_all(&CHUNK_NAME_LOWER)?;
		}

		file.write_u32::<B>(tag.len() as u32)?;
		file.write_all(tag)?;

		// It is required an odd length chunk be padded with a 0
		// The 0 isn't included in the chunk size, however
		if tag.len() % 2 != 0 {
			file.write_u8(0)?;
		}

		let total_size = file.stream_position()? - 8;

		file.seek(SeekFrom::Start(4))?;

		file.write_u32::<B>(total_size as u32)?;
	}

	Ok(())
}
