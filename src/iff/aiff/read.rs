#[cfg(feature = "aiff_text_chunks")]
use super::tag::{AiffTextChunks, Comment};
use super::AiffFile;
use crate::error::{LoftyError, Result};
#[cfg(feature = "id3v2")]
use crate::id3::v2::tag::Id3v2Tag;
use crate::iff::chunk::Chunks;
use crate::types::properties::FileProperties;

use std::io::{Read, Seek, SeekFrom};

use byteorder::{BigEndian, ReadBytesExt};

pub(in crate::iff) fn verify_aiff<R>(data: &mut R) -> Result<()>
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

pub(crate) fn read_from<R>(data: &mut R, read_properties: bool) -> Result<AiffFile>
where
	R: Read + Seek,
{
	verify_aiff(data)?;

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

	let mut chunks = Chunks::<BigEndian>::new();

	while chunks.next(data).is_ok() {
		match &chunks.fourcc {
			#[cfg(feature = "id3v2")]
			b"ID3 " | b"id3 " => id3v2_tag = Some(chunks.id3_chunk(data)?),
			b"COMM" if read_properties && comm.is_none() => {
				if chunks.size < 18 {
					return Err(LoftyError::Aiff(
						"File has an invalid \"COMM\" chunk size (< 18)",
					));
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
				annotations.push(chunks.read_string(data)?);
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

					let mut text = vec![0; size as usize];
					data.read_exact(&mut text)?;

					comments.push(Comment {
						timestamp,
						marker_id,
						text: String::from_utf8(text)?,
					})
				}

				chunks.correct_position(data)?;
			},
			#[cfg(feature = "aiff_text_chunks")]
			b"NAME" if text_chunks.name.is_none() => {
				text_chunks.name = Some(chunks.read_string(data)?);
			},
			#[cfg(feature = "aiff_text_chunks")]
			b"AUTH" if text_chunks.author.is_none() => {
				text_chunks.author = Some(chunks.read_string(data)?);
			},
			#[cfg(feature = "aiff_text_chunks")]
			b"(c) " if text_chunks.copyright.is_none() => {
				text_chunks.copyright = Some(chunks.read_string(data)?);
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

	let properties = if read_properties {
		if comm.is_none() {
			return Err(LoftyError::Aiff("File does not contain a \"COMM\" chunk"));
		}

		if stream_len == 0 {
			return Err(LoftyError::Aiff("File does not contain a \"SSND\" chunk"));
		}

		super::properties::read_properties(
			&mut &*comm.unwrap(),
			stream_len,
			data.seek(SeekFrom::Current(0))?,
		)?
	} else {
		FileProperties::default()
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
