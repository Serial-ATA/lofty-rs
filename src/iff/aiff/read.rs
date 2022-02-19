#[cfg(feature = "aiff_text_chunks")]
use super::tag::{AiffTextChunks, Comment};
use super::AiffFile;
use crate::error::{ErrorKind, FileDecodingError, LoftyError, Result};
#[cfg(feature = "id3v2")]
use crate::id3::v2::tag::Id3v2Tag;
use crate::iff::chunk::Chunks;
use crate::types::file::FileType;
use crate::types::properties::FileProperties;

use std::io::{Read, Seek, SeekFrom};

use byteorder::{BigEndian, ReadBytesExt};

pub(in crate::iff) fn verify_aiff<R>(data: &mut R) -> Result<u32>
where
	R: Read + Seek,
{
	let mut id = [0; 12];
	data.read_exact(&mut id)?;

	if !(&id[..4] == b"FORM" && (&id[8..] == b"AIFF" || &id[..8] == b"AIFC")) {
		return Err(LoftyError::new(ErrorKind::UnknownFormat));
	}

	Ok(u32::from_be_bytes(
		id[4..8].try_into().unwrap(), // Infallible
	))
}

pub(crate) fn read_from<R>(data: &mut R, read_properties: bool) -> Result<AiffFile>
where
	R: Read + Seek,
{
	let file_size = verify_aiff(data)?;

	let mut comm = None;
	let mut stream_len = 0;

	#[cfg(feature = "aiff_text_chunks")]
	let mut text_chunks = AiffTextChunks::default();
	#[cfg(feature = "aiff_text_chunks")]
	let mut annotations = Vec::new();
	#[cfg(feature = "aiff_text_chunks")]
	let mut comments = Vec::new();

	#[cfg(feature = "id3v2")]
	let mut id3v2_tag: Option<Id3v2Tag> = None;

	let mut chunks = Chunks::<BigEndian>::new(file_size);

	while chunks.next(data).is_ok() {
		match &chunks.fourcc {
			#[cfg(feature = "id3v2")]
			b"ID3 " | b"id3 " => id3v2_tag = Some(chunks.id3_chunk(data)?),
			b"COMM" if read_properties && comm.is_none() => {
				if chunks.size < 18 {
					return Err(FileDecodingError::new(
						FileType::AIFF,
						"File has an invalid \"COMM\" chunk size (< 18)",
					)
					.into());
				}

				comm = Some(chunks.content(data)?);
				chunks.correct_position(data)?;
			},
			b"SSND" if read_properties => {
				stream_len = chunks.size;
				chunks.skip(data)?;
			},
			#[cfg(feature = "aiff_text_chunks")]
			b"ANNO" => {
				annotations.push(chunks.read_pstring(data, None)?);
			},
			// These four chunks are expected to appear at most once per file,
			// so there's no need to replace anything we already read
			#[cfg(feature = "aiff_text_chunks")]
			b"COMT" if comments.is_empty() => {
				let num_comments = data.read_u16::<BigEndian>()?;

				for _ in 0..num_comments {
					let timestamp = data.read_u32::<BigEndian>()?;
					let marker_id = data.read_u16::<BigEndian>()?;
					let size = data.read_u16::<BigEndian>()?;

					let text = chunks.read_pstring(data, Some(u32::from(size)))?;

					comments.push(Comment {
						timestamp,
						marker_id,
						text,
					})
				}

				chunks.correct_position(data)?;
			},
			#[cfg(feature = "aiff_text_chunks")]
			b"NAME" if text_chunks.name.is_none() => {
				text_chunks.name = Some(chunks.read_pstring(data, None)?);
			},
			#[cfg(feature = "aiff_text_chunks")]
			b"AUTH" if text_chunks.author.is_none() => {
				text_chunks.author = Some(chunks.read_pstring(data, None)?);
			},
			#[cfg(feature = "aiff_text_chunks")]
			b"(c) " if text_chunks.copyright.is_none() => {
				text_chunks.copyright = Some(chunks.read_pstring(data, None)?);
			},
			_ => chunks.skip(data)?,
		}
	}

	#[cfg(feature = "aiff_text_chunks")]
	{
		if !annotations.is_empty() {
			text_chunks.annotations = Some(annotations);
		}

		if !comments.is_empty() {
			text_chunks.comments = Some(comments);
		}
	}

	let properties;
	if read_properties {
		match comm {
			Some(comm) => {
				if stream_len == 0 {
					return Err(FileDecodingError::new(
						FileType::AIFF,
						"File does not contain a \"SSND\" chunk",
					)
					.into());
				}

				properties = super::properties::read_properties(
					&mut &*comm,
					stream_len,
					data.seek(SeekFrom::Current(0))?,
				)?;
			},
			None => {
				return Err(FileDecodingError::new(
					FileType::AIFF,
					"File does not contain a \"COMM\" chunk",
				)
				.into());
			},
		}
	} else {
		properties = FileProperties::default();
	};

	Ok(AiffFile {
		properties,
		#[cfg(feature = "aiff_text_chunks")]
		text_chunks: match text_chunks {
			AiffTextChunks {
				name: None,
				author: None,
				copyright: None,
				annotations: None,
				comments: None,
			} => None,
			_ => Some(text_chunks),
		},
		#[cfg(feature = "id3v2")]
		id3v2_tag,
	})
}
