use crate::{FileProperties, LoftyError, Result};

use byteorder::{BigEndian, LittleEndian, ReadBytesExt};

use std::cmp::{max, min};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::time::Duration;

fn verify_aiff<T>(data: &mut T) -> Result<()>
where
	T: Read + Seek,
{
	let mut id = [0; 12];
	data.read_exact(&mut id)?;

	if !(&id[..4] == b"FORM" && (&id[8..] == b"AIFF" || &id[..8] == b"AIFC")) {
		return Err(LoftyError::UnknownFormat);
	}

	Ok(())
}

pub(crate) fn read_properties<R>(data: &mut R) -> Result<FileProperties>
where
	R: Read + Seek,
{
	verify_aiff(data)?;

	let mut comm = None;
	let mut stream_len = 0;

	let start = data.seek(SeekFrom::Current(0))?;

	while let (Ok(fourcc), Ok(size)) = (
		data.read_u32::<LittleEndian>(),
		data.read_u32::<BigEndian>(),
	) {
		if comm.is_some() && stream_len > 0 {
			break;
		}

		match &fourcc.to_le_bytes() {
			b"COMM" => {
				if comm.is_none() {
					if size < 18 {
						return Err(LoftyError::InvalidData(
							"AIFF file has an invalid COMM chunk size (< 18)",
						));
					}

					let mut comm_data = vec![0; size as usize];
					data.read_exact(&mut comm_data)?;

					comm = Some(comm_data);
				}
			},
			b"SSND" => stream_len = size,
			_ => {
				data.seek(SeekFrom::Current(i64::from(size)))?;
			},
		}
	}

	data.seek(SeekFrom::Start(start))?;

	if comm.is_none() {
		return Err(LoftyError::InvalidData(
			"AIFF file does not contain a COMM chunk",
		));
	}

	if stream_len == 0 {
		return Err(LoftyError::InvalidData(
			"AIFF file does not contain a SSND chunk",
		));
	}

	let comm = &mut &*comm.unwrap();

	let channels = comm.read_u16::<BigEndian>()? as u8;
	let sample_frames = comm.read_u32::<BigEndian>()?;
	let _sample_size = comm.read_u16::<BigEndian>()?;

	let mut sample_rate_bytes = [0; 10];
	comm.read_exact(&mut sample_rate_bytes)?;

	let sign = u64::from(sample_rate_bytes[0] & 0x80);

	sample_rate_bytes[0] &= 0x7f;

	let mut exponent = u16::from(sample_rate_bytes[0]) << 8 | u16::from(sample_rate_bytes[1]);
	exponent = exponent - 16383 + 1023;

	let fraction = &mut sample_rate_bytes[2..];
	fraction[0] &= 0x7f;

	let fraction: Vec<u64> = fraction.iter_mut().map(|v| u64::from(*v)).collect();

	let fraction = fraction[0] << 56
		| fraction[1] << 48
		| fraction[2] << 40
		| fraction[3] << 32
		| fraction[4] << 24
		| fraction[5] << 16
		| fraction[6] << 8
		| fraction[7];

	let f64_bytes = sign << 56 | u64::from(exponent) << 52 | fraction >> 11;
	let float = f64::from_be_bytes(f64_bytes.to_be_bytes());

	let sample_rate = float.round() as u32;

	let (duration, bitrate) = if sample_rate > 0 && sample_frames > 0 {
		let length = (u64::from(sample_frames) * 1000) / u64::from(sample_rate);

		(
			Duration::from_millis(length),
			(u64::from(stream_len * 8) / length) as u32,
		)
	} else {
		(Duration::ZERO, 0)
	};

	Ok(FileProperties::new(
		duration,
		Some(bitrate),
		Some(sample_rate),
		Some(channels),
	))
}

cfg_if::cfg_if! {
	if #[cfg(feature = "format-aiff")] {
		type AiffTags = (
			Option<String>,
			Option<String>,
			Option<String>,
			FileProperties,
		);

		pub(crate) fn read_from<T>(data: &mut T) -> Result<AiffTags>
		where
			T: Read + Seek,
		{
			let mut name_id: Option<String> = None;
			let mut author_id: Option<String> = None;
			let mut copyright_id: Option<String> = None;

			let properties = read_properties(data)?;

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

			Ok((name_id, author_id, copyright_id, properties))
		}

		pub(crate) fn write_to(
			data: &mut File,
			metadata: (Option<&String>, Option<&String>, Option<&String>),
		) -> Result<()> {
			verify_aiff(data)?;

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
	}
}
