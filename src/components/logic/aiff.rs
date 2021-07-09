use crate::{LoftyError, Result};

use byteorder::{BigEndian, LittleEndian, ReadBytesExt};

use std::cmp::{max, min};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};

pub(crate) fn read_from<T>(data: &mut T) -> Result<(Option<String>, Option<String>, Option<String>)>
where
	T: Read + Seek,
{
	let mut name_id: Option<String> = None;
	let mut author_id: Option<String> = None;
	let mut copyright_id: Option<String> = None;

	data.seek(SeekFrom::Start(12))?;

	while let (Ok(fourcc), Ok(size)) = (
		data.read_u32::<LittleEndian>(),
		data.read_u32::<BigEndian>(),
	) {
		match &fourcc.to_le_bytes() {
			f if f == b"NAME" && name_id.is_none() => {
				let mut name = vec![0; size as usize];
				data.read_exact(&mut name)?;

				name_id = Some(String::from_utf8(name)?);
			},
			f if f == b"AUTH" && author_id.is_none() => {
				let mut auth = vec![0; size as usize];
				data.read_exact(&mut auth)?;

				author_id = Some(String::from_utf8(auth)?);
			},
			f if f == b"(c) " && copyright_id.is_none() => {
				let mut copy = vec![0; size as usize];
				data.read_exact(&mut copy)?;

				copyright_id = Some(String::from_utf8(copy)?);
			},
			_ => {
				data.seek(SeekFrom::Current(i64::from(size)))?;
			},
		}
	}

	if (&None, &None, &None) == (&name_id, &author_id, &copyright_id) {
		return Err(LoftyError::InvalidData("AIFF file contains no text chunks"));
	}

	Ok((name_id, author_id, copyright_id))
}

pub(crate) fn write_to(
	data: &mut File,
	metadata: (Option<&String>, Option<&String>, Option<&String>),
) -> Result<()> {
	let mut text_chunks = Vec::new();

	if let Some(name_id) = metadata.0 {
		let len = (name_id.len() as u32).to_be_bytes();

		text_chunks.extend(b"NAME".iter());
		text_chunks.extend(len.iter());
		text_chunks.extend(name_id.as_bytes().iter());
	}

	if let Some(author_id) = metadata.1 {
		let len = (author_id.len() as u32).to_be_bytes();

		text_chunks.extend(b"AUTH".iter());
		text_chunks.extend(len.iter());
		text_chunks.extend(author_id.as_bytes().iter());
	}

	if let Some(copyright_id) = metadata.2 {
		let len = (copyright_id.len() as u32).to_be_bytes();

		text_chunks.extend(b"(c) ".iter());
		text_chunks.extend(len.iter());
		text_chunks.extend(copyright_id.as_bytes().iter());
	}

	data.seek(SeekFrom::Start(12))?;

	let mut name: Option<(usize, usize)> = None;
	let mut auth: Option<(usize, usize)> = None;
	let mut copy: Option<(usize, usize)> = None;

	while let (Ok(fourcc), Ok(size)) = (
		data.read_u32::<LittleEndian>(),
		data.read_u32::<BigEndian>(),
	) {
		let pos = (data.seek(SeekFrom::Current(0))? - 8) as usize;

		match &fourcc.to_le_bytes() {
			f if f == b"NAME" && name.is_none() => name = Some((pos, (pos + 8 + size as usize))),
			f if f == b"AUTH" && auth.is_none() => auth = Some((pos, (pos + 8 + size as usize))),
			f if f == b"(c) " && copy.is_none() => copy = Some((pos, (pos + 8 + size as usize))),
			_ => {
				data.seek(SeekFrom::Current(i64::from(size)))?;
				continue;
			},
		}

		data.seek(SeekFrom::Current(i64::from(size)))?;
	}

	data.seek(SeekFrom::Start(0))?;

	let mut file_bytes = Vec::new();
	data.read_to_end(&mut file_bytes)?;

	match (name, auth, copy) {
		(None, None, None) => {
			data.seek(SeekFrom::Start(16))?;

			let mut size = [0; 4];
			data.read_exact(&mut size)?;

			let comm_end = (20 + u32::from_le_bytes(size)) as usize;
			file_bytes.splice(comm_end..comm_end, text_chunks);
		},
		(Some(single_value), None, None)
		| (None, Some(single_value), None)
		| (None, None, Some(single_value)) => {
			file_bytes.splice(single_value.0..single_value.1, text_chunks);
		},
		#[rustfmt::skip]
		(Some(a), Some(b), None)
		| (Some(a), None, Some(b))
		| (None, Some(a), Some(b)) => {
			let first = min(a, b);
			let end = max(a, b);

			file_bytes.drain(end.0..end.1);
			file_bytes.splice(first.0..first.1, text_chunks);
		},
		(Some(title), Some(author), Some(copyright)) => {
			let mut items = vec![title, author, copyright];
			items.sort_unstable();

			let first = items[0];
			let mid = items[1];
			let end = items[2];

			file_bytes.drain(end.0..end.1);
			file_bytes.drain(mid.0..mid.1);
			file_bytes.splice(first.0..first.1, text_chunks);
		},
	}

	let total_size = ((file_bytes.len() - 8) as u32).to_be_bytes();
	file_bytes.splice(4..8, total_size.to_vec());

	data.seek(SeekFrom::Start(0))?;
	data.set_len(0)?;
	data.write_all(&*file_bytes)?;

	Ok(())
}
