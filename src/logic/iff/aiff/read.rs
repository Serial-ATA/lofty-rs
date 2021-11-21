#[cfg(feature = "aiff_text_chunks")]
use super::tag::AiffTextChunks;
use super::AiffFile;
use crate::error::{LoftyError, Result};
#[cfg(feature = "id3v2")]
use crate::logic::id3::v2::read::parse_id3v2;
#[cfg(feature = "id3v2")]
use crate::logic::id3::v2::tag::Id3v2Tag;

use std::io::{Read, Seek, SeekFrom};

use byteorder::{BigEndian, ReadBytesExt};

pub(in crate::logic::iff) fn verify_aiff<R>(data: &mut R) -> Result<()>
where
	R: Read + Seek,
{
	let mut id = [0; 12];
	data.read_exact(&mut id)?;

	if !(&id[..4] == b"FORM" && (&id[8..] == b"AIFF" || &id[..8] == b"AIFC")) {
		return Err(LoftyError::UnknownFormat);
	}

	Ok(())
}

pub(in crate::logic) fn read_from<R>(data: &mut R) -> Result<AiffFile>
where
	R: Read + Seek,
{
	verify_aiff(data)?;

	let mut comm = None;
	let mut stream_len = 0;

	#[cfg(feature = "aiff_text_chunks")]
	let mut text_chunks = AiffTextChunks::default();
	#[cfg(feature = "id3v2")]
	let mut id3v2_tag: Option<Id3v2Tag> = None;

	let mut fourcc = [0; 4];

	while let (Ok(()), Ok(size)) = (data.read_exact(&mut fourcc), data.read_u32::<BigEndian>()) {
		match &fourcc {
			#[cfg(feature = "aiff_text_chunks")]
			b"NAME" | b"AUTH" | b"(c) " => {
				let mut value = vec![0; size as usize];
				data.read_exact(&mut value)?;

				let value = String::from_utf8(value)?;

				match &fourcc {
					b"NAME" => text_chunks.name = Some(value),
					b"AUTH" => text_chunks.author = Some(value),
					b"(c) " => text_chunks.copyright = Some(value),
					_ => unreachable!(),
				}
			}
			#[cfg(feature = "id3v2")]
			b"ID3 " | b"id3 " => {
				let mut value = vec![0; size as usize];
				data.read_exact(&mut value)?;

				let id3v2 = parse_id3v2(&mut &*value)?;

				// Skip over the footer
				if id3v2.flags().footer {
					data.seek(SeekFrom::Current(10))?;
				}

				id3v2_tag = Some(id3v2);
			}
			b"COMM" => {
				if comm.is_none() {
					if size < 18 {
						return Err(LoftyError::Aiff(
							"File has an invalid \"COMM\" chunk size (< 18)",
						));
					}

					let mut comm_data = vec![0; size as usize];
					data.read_exact(&mut comm_data)?;

					comm = Some(comm_data);
				}
			}
			b"SSND" => {
				stream_len = size;
				data.seek(SeekFrom::Current(i64::from(size)))?;
			}
			_ => {
				data.seek(SeekFrom::Current(i64::from(size)))?;
			}
		}

		// Chunks only start on even boundaries
		if size % 2 != 0 {
			data.seek(SeekFrom::Current(1))?;
		}
	}

	if comm.is_none() {
		return Err(LoftyError::Aiff("File does not contain a \"COMM\" chunk"));
	}

	if stream_len == 0 {
		return Err(LoftyError::Aiff("File does not contain a \"SSND\" chunk"));
	}

	let properties = super::properties::read_properties(&mut &*comm.unwrap(), stream_len)?;

	Ok(AiffFile {
		properties,
		#[cfg(feature = "aiff_text_chunks")]
		text_chunks: match text_chunks {
			AiffTextChunks {
				name: None,
				author: None,
				copyright: None,
			} => None,
			_ => Some(text_chunks),
		},
		#[cfg(feature = "id3v2")]
		id3v2_tag,
	})
}
