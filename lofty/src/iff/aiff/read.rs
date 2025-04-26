use super::AiffFile;
use super::properties::AiffProperties;
use super::tag::{AiffTextChunks, Comment};
use crate::config::ParseOptions;
use crate::error::Result;
use crate::id3::v2::tag::Id3v2Tag;
use crate::iff::chunk::Chunks;
use crate::macros::{decode_err, err};

use std::io::{Read, Seek, SeekFrom};

use byteorder::{BigEndian, ReadBytesExt};

/// Whether we are dealing with an AIFC file
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(in crate::iff) enum CompressionPresent {
	Yes,
	No,
}

pub(in crate::iff) fn verify_aiff<R>(data: &mut R) -> Result<CompressionPresent>
where
	R: Read + Seek,
{
	let mut id = [0; 12];
	data.read_exact(&mut id)?;

	if &id[..4] != b"FORM" {
		err!(UnknownFormat);
	}

	let compression_present;
	match &id[8..] {
		b"AIFF" => compression_present = CompressionPresent::No,
		b"AIFC" => compression_present = CompressionPresent::Yes,
		_ => err!(UnknownFormat),
	}

	log::debug!(
		"File verified to be AIFF, compression present: {:?}",
		compression_present
	);
	Ok(compression_present)
}

pub(crate) fn read_from<R>(data: &mut R, parse_options: ParseOptions) -> Result<AiffFile>
where
	R: Read + Seek,
{
	// TODO: Maybe one day the `Seek` bound can be removed?
	// let file_size = verify_aiff(data)?;
	let compression_present = verify_aiff(data)?;

	let current_pos = data.stream_position()?;
	let file_len = data.seek(SeekFrom::End(0))?;

	data.seek(SeekFrom::Start(current_pos))?;

	let mut comm = None;
	let mut stream_len = 0;

	let mut text_chunks = AiffTextChunks::default();
	let mut annotations = Vec::new();
	let mut comments = Vec::new();

	let mut id3v2_tag: Option<Id3v2Tag> = None;

	let mut chunks = Chunks::<BigEndian>::new(file_len);

	while let Ok(true) = chunks.next(data) {
		match &chunks.fourcc {
			b"ID3 " | b"id3 " if parse_options.read_tags => {
				let tag = chunks.id3_chunk(data, parse_options)?;
				if let Some(existing_tag) = id3v2_tag.as_mut() {
					log::warn!("Duplicate ID3v2 tag found, appending frames to previous tag");

					// https://github.com/Serial-ATA/lofty-rs/issues/87
					// Duplicate tags should have their frames appended to the previous
					for frame in tag.frames {
						existing_tag.insert(frame);
					}
					continue;
				}
				id3v2_tag = Some(tag);
			},
			b"COMM" if parse_options.read_properties && comm.is_none() => {
				if chunks.size < 18 {
					decode_err!(@BAIL Aiff, "File has an invalid \"COMM\" chunk size (< 18)");
				}

				comm = Some(chunks.content(data)?);
				chunks.correct_position(data)?;
			},
			b"SSND" if parse_options.read_properties => {
				stream_len = chunks.size;
				chunks.skip(data)?;
			},
			b"ANNO" if parse_options.read_tags => {
				annotations.push(chunks.read_pstring(data, None)?);
			},
			// These four chunks are expected to appear at most once per file,
			// so there's no need to replace anything we already read
			b"COMT" if comments.is_empty() && parse_options.read_tags => {
				if chunks.size < 2 {
					continue;
				}

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
			b"NAME" if text_chunks.name.is_none() && parse_options.read_tags => {
				text_chunks.name = Some(chunks.read_pstring(data, None)?);
			},
			b"AUTH" if text_chunks.author.is_none() && parse_options.read_tags => {
				text_chunks.author = Some(chunks.read_pstring(data, None)?);
			},
			b"(c) " if text_chunks.copyright.is_none() && parse_options.read_tags => {
				text_chunks.copyright = Some(chunks.read_pstring(data, None)?);
			},
			_ => chunks.skip(data)?,
		}
	}

	if !annotations.is_empty() {
		text_chunks.annotations = Some(annotations);
	}

	if !comments.is_empty() {
		text_chunks.comments = Some(comments);
	}

	let properties;
	if parse_options.read_properties {
		match comm {
			Some(comm) => {
				if stream_len == 0 {
					decode_err!(@BAIL Aiff, "File does not contain a \"SSND\" chunk");
				}

				properties = super::properties::read_properties(
					&mut &*comm,
					compression_present,
					stream_len,
					data.stream_position()?,
				)?;
			},
			None => decode_err!(@BAIL Aiff, "File does not contain a \"COMM\" chunk"),
		}
	} else {
		properties = AiffProperties::default();
	}

	Ok(AiffFile {
		properties,
		text_chunks_tag: match text_chunks {
			AiffTextChunks {
				name: None,
				author: None,
				copyright: None,
				annotations: None,
				comments: None,
			} => None,
			_ => Some(text_chunks),
		},
		id3v2_tag,
	})
}
